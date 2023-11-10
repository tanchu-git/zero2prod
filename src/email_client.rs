use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

use crate::domain::subscriber_email::SubscriberEmail;

#[derive(Debug)]
pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    secret: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        secret: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            http_client: Client::builder().timeout(timeout).build().unwrap(),
            base_url,
            sender,
            secret,
        }
    }

    pub async fn send_email(&self, recipient: &SubscriberEmail) -> Result<(), reqwest::Error> {
        let url = format!("{}/campaigns/9b4079798b/actions/test", self.base_url);
        let request_body = SendEmailRequestBody {
            test_emails: vec![recipient.as_ref()],
            send_type: "html",
        };
        let builder = self
            .http_client
            .post(url)
            .header("authorization", self.secret.expose_secret())
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        dbg!(builder);
        dbg!(&self.sender);

        Ok(())
    }
}

#[derive(serde::Serialize)]
struct SendEmailRequestBody<'a> {
    test_emails: Vec<&'a str>,
    send_type: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::domain::subscriber_email::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            // Try to parse the body as a JSON value
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                // Check that all the mandatory fields are populated
                // without inspecting the field values
                body.get("test_emails").is_some() && body.get("send_type").is_some()
            } else {
                // If parsing failed, do not match the request
                false
            }
        }
    }

    async fn mock_setup() -> (MockServer, EmailClient, SubscriberEmail) {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        );
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();

        (mock_server, email_client, subscriber_email)
    }

    #[actix_web::test]
    async fn send_email_sends_the_expected_request() {
        let (mock_server, email_client, subscriber_email) = mock_setup().await;

        Mock::given(header_exists("authorization"))
            .and(path("campaigns/9b4079798b/actions/test"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client.send_email(&subscriber_email).await;
    }

    #[actix_web::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let (mock_server, email_client, subscriber_email) = mock_setup().await;

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client.send_email(&subscriber_email).await;

        assert_ok!(outcome);
    }

    #[actix_web::test]
    async fn send_email_succeeds_if_the_server_returns_500() {
        let (mock_server, email_client, subscriber_email) = mock_setup().await;

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client.send_email(&subscriber_email).await;

        assert_err!(outcome);
    }

    #[actix_web::test]
    async fn send_email_times_out_if_server_takes_too_long() {
        let (mock_server, email_client, subscriber_email) = mock_setup().await;

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client.send_email(&subscriber_email).await;

        assert_err!(outcome);
    }
}
