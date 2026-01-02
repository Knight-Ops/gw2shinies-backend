#[derive(Clone)]
pub struct Gw2Client {
    client: reqwest::Client,
    gw2_url: String,
    bltc_url: String,
}

impl Gw2Client {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            gw2_url: "https://api.guildwars2.com".to_string(),
            bltc_url: "https://www.gw2bltc.com".to_string(),
        }
    }

    #[cfg(test)]
    pub fn with_urls(gw2_url: String, bltc_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            gw2_url,
            bltc_url,
        }
    }

    pub async fn fetch_all_item_ids(&self) -> Result<Vec<u32>, reqwest::Error> {
        let url = format!("{}/v2/items", self.gw2_url);
        let ids = self
            .client
            .get(url)
            .send()
            .await?
            .json::<Vec<u32>>()
            .await?;
        Ok(ids)
    }

    pub async fn fetch_items_chunk(
        &self,
        ids: &[u32],
    ) -> Result<Vec<crate::item_definition::ItemDefinition>, reqwest::Error> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let url = format!("{}/v2/items?ids={}", self.gw2_url, ids_str);
        let items = self
            .client
            .get(url)
            .send()
            .await?
            .json::<Vec<crate::item_definition::RawItem>>()
            .await?;

        Ok(items.into_iter().map(|i| i.into()).collect())
    }

    pub async fn fetch_all_price_ids(&self) -> Result<Vec<u32>, reqwest::Error> {
        let url = format!("{}/v2/commerce/prices", self.gw2_url);
        let ids = self
            .client
            .get(url)
            .send()
            .await?
            .json::<Vec<u32>>()
            .await?;
        Ok(ids)
    }

    pub async fn fetch_prices_chunk(
        &self,
        ids: &[u32],
    ) -> Result<Vec<crate::history_record::HistoryRecord>, reqwest::Error> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let url = format!("{}/v2/commerce/prices?ids={}", self.gw2_url, ids_str);
        let prices = self
            .client
            .get(url)
            .send()
            .await?
            .json::<Vec<crate::history_record::RawPrice>>()
            .await?;

        let now = chrono::Utc::now();
        Ok(prices
            .into_iter()
            .map(|p| crate::history_record::HistoryRecord::from_raw(p, now))
            .collect())
    }

    pub async fn fetch_item_history(
        &self,
        id: u32,
    ) -> Result<Vec<crate::history_record::HistoryRecord>, reqwest::Error> {
        let url = format!("{}/api/tp/chart/{}", self.bltc_url, id);
        let response = self.client.get(url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }

        let raw_history = response.error_for_status()?.json::<Vec<Vec<i64>>>().await?;

        Ok(raw_history
            .into_iter()
            .filter_map(|data| crate::history_record::HistoryRecord::from_bltc(id, &data))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_fetch_all_item_ids() {
        let server = MockServer::start().await;
        let mock_ids = vec![1, 2, 3];
        
        Mock::given(method("GET"))
            .and(path("/v2/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_ids))
            .mount(&server)
            .await;

        let client = Gw2Client::with_urls(server.uri(), "".to_string());
        let ids = client.fetch_all_item_ids().await.unwrap();
        
        assert_eq!(ids, mock_ids);
    }

    #[tokio::test]
    async fn test_fetch_item_history() {
        let server = MockServer::start().await;
        let item_id = 19684;
        let mock_data = vec![
            vec![1735689600, 60, 50, 200, 100],
        ];

        Mock::given(method("GET"))
            .and(path(format!("/api/tp/chart/{}", item_id)))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_data))
            .mount(&server)
            .await;

        let client = Gw2Client::with_urls("".to_string(), server.uri());
        let history = client.fetch_item_history(item_id).await.unwrap();
        
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].sell_price, 60);
    }

    #[tokio::test]
    async fn test_fetch_item_history_not_found() {
        let server = MockServer::start().await;
        let item_id = 12345;

        Mock::given(method("GET"))
            .and(path(format!("/api/tp/chart/{}", item_id)))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = Gw2Client::with_urls("".to_string(), server.uri());
        let history = client.fetch_item_history(item_id).await.unwrap();
        
        assert!(history.is_empty());
    }
}
