use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        let is_empty_or_whitespace = s.trim().is_empty();

        let is_too_long = s.graphemes(true).count() > 256;

        let forbiden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        let contains_forbiden_characters = s.chars().any(|g| forbiden_characters.contains(&g));

        if is_empty_or_whitespace || contains_forbiden_characters || is_too_long {
            Err(format!("{} is not a valid subscriber name", s))
        } else {
            Ok(Self(s))
        }
    }

    pub fn inner(self) -> String {
        self.0
    }

    pub fn inner_mut(&mut self) -> &mut str {
        &mut self.0
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};

    use crate::domain::SubscriberName;

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a".repeat(256);

        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_that_256_grapheme_is_rejected() {
        let name = "a".repeat(257);

        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn withespace_only_name_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn subscriber_name_contains_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parse_sucessfully() {
        let name = "ursula le guin".to_string();

        assert_ok!(SubscriberName::parse(name));
    }
}
