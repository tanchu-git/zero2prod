use super::{subscriber_email::SubscriberEmail, subscriber_name::SubscriberName};

pub struct NewSubscriber {
    email: SubscriberEmail,
    name: SubscriberName,
}

impl NewSubscriber {
    pub fn new(email: SubscriberEmail, name: SubscriberName) -> Self {
        Self { email, name }
    }
    pub fn get_email(&self) -> &str {
        self.email.as_ref()
    }

    pub fn get_name(&self) -> &str {
        self.name.as_ref()
    }
}
