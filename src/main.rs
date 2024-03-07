use reqwest::StatusCode;
use scraper::{Html, Selector};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use std::io::stdin;
use url::Url;

const BASE_URL: &str = "https://packages.fedoraproject.org/pkgs";

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error("not a base URL")]
    NotABase,
    #[error("selector error: {0}")]
    Selector(String),
}

#[derive(Clone, Debug)]
struct Version {
    pub distribution: String,
    pub version: String,
}

struct Client {
    url: Url,
    client: reqwest::Client,
}

impl Client {
    pub fn new(url: Url, client: reqwest::Client) -> Self {
        Self { url, client }
    }

    pub async fn scrape(&self, p: &str, s: &str) -> Result<Vec<Version>, Error> {
        let mut url = self.url.clone();
        url.path_segments_mut()
            .map_err(|()| Error::NotABase)?
            .push(p)
            .push(s);

        let response = self.client.get(url).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }
        let response = response.error_for_status()?.text().await?;

        let html = Html::parse_document(&response);

        let selector = selector(r#"table#version-table > tbody > tr"#)?;
        let result = html.select(&selector);

        Ok(result
            .flat_map(|element| {
                let mut children = element.child_elements();
                match (children.next(), children.next()) {
                    (Some(dist), Some(version)) => Some(Version {
                        distribution: dist.text().collect::<String>().trim().to_string(),
                        version: version.text().collect::<String>().trim().to_string(),
                    }),
                    _ => None,
                }
            })
            .collect())
    }
}

fn selector(s: &str) -> Result<Selector, Error> {
    Selector::parse(s).map_err(|err| Error::Selector(err.to_string()))
}

impl Default for Client {
    fn default() -> Self {
        Self::new(Url::parse(BASE_URL).unwrap(), reqwest::Client::new())
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let client = Client::default();

    let mut missing = 0;

    while let Some(Ok(line)) = stdin().lines().next() {
        let Some((c, v)) = line.split_once(" ") else {
            continue;
        };

        let p = format!("rust-{c}");
        let s = format!("{p}-devel");
        let mut result = client.scrape(&p, &s).await?;
        result.retain(|item| item.distribution == "Fedora Rawhide");
        if result.is_empty() {
            missing += 1;
            log::warn!("{c}: missing");
        } else {
            log::info!("{c}: found: {result:?}");
        }

        log::info!("{c}: result: {result:?}");
    }

    log::info!("{missing} missing packages");

    Ok(())
}
