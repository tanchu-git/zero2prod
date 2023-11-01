use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(email: String) -> Result<SubscriberEmail, String> {
        if validate_email(&email) {
            Ok(Self(email))
        } else {
            Err(format!("{email} is not a valid email."))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use rand::{rngs::StdRng, SeedableRng};
    use rstest::rstest;

    // Both `Clone` and `Debug` are required by `quickcheck`
    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[rstest]
    #[case("ursuladomain.com", "email missing at symbol is rejected")]
    #[case("", "empty string is rejected")]
    #[case("@domain.com", "email missing subject is rejected")]
    #[trace]
    fn test_email_validation(#[case] email: String, #[case] test_case: &str) {
        assert_err!(SubscriberEmail::parse(email), "case: {test_case}");
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
