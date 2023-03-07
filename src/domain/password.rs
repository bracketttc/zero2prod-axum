use anyhow::anyhow;
use derive_more::{Deref, DerefMut};
use passwords::analyzer;
use secrecy::{ExposeSecret, Secret};

#[derive(Deref, DerefMut, Debug)]
pub struct Password(Secret<String>);

impl Password {
    pub fn parse(input: Secret<String>) -> Result<Self, anyhow::Error> {
        let analysis = analyzer::analyze(input.expose_secret());
        if analysis.password() != input.expose_secret() {
            return Err(anyhow!("Password contains invalid characters."));
        }
        if analysis.length() < 12 {
            return Err(anyhow!("Password is shorter than 12 characters."));
        }
        if analysis.length() > 128 {
            return Err(anyhow!("Password is longer than 128 characters."));
        }
        Ok(Self(input))
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::Password;
    use claims::assert_err;
    use fake::Fake;
    use secrecy::Secret;

    #[test]
    fn passwords_with_invalid_characters_are_rejected() {
        assert_err!(Password::parse(Secret::new("\rPassword123".to_owned())));
    }

    #[test]
    fn passwords_shorter_than_12_characters_are_rejected() {
        assert_err!(Password::parse(Secret::new(
            fake::faker::internet::en::Password(1..11).fake()
        )));
    }
}
