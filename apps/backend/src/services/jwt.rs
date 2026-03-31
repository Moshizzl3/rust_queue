use chrono::{Duration, Utc};
use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode, errors::Error,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshClaims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub token_type: String,
}

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_expiry_mins: i64,
    refresh_expiry_days: i64,
}

impl JwtService {
    pub fn new(secret: &str, access_expiry_mins: i64, refresh_expiry_days: i64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_expiry_mins,
            refresh_expiry_days,
        }
    }

    pub fn generate_access_token(&self, user_id: Uuid, email: &str) -> Result<String, Error> {
        let now = Utc::now();
        let exp = now + Duration::minutes(self.access_expiry_mins);
        let claims = AccessClaims {
            sub: user_id,
            email: email.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type: "access".to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
    }

    pub fn generate_refresh_token(&self, user_id: Uuid) -> Result<String, Error> {
        let now = Utc::now();
        let exp = now + Duration::days(self.refresh_expiry_days);
        let claims = RefreshClaims {
            sub: user_id,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type: "refresh".to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
    }

    pub fn generate_token_pair(
        &self,
        user_id: Uuid,
        email: &str,
    ) -> Result<(String, String), Error> {
        let access_token = self.generate_access_token(user_id, email)?;
        let refresh_token = self.generate_refresh_token(user_id)?;
        Ok((access_token, refresh_token))
    }

    pub fn validate_access_token(&self, token: &str) -> Result<TokenData<AccessClaims>, Error> {
        let token_data = decode::<AccessClaims>(token, &self.decoding_key, &Validation::default())?;

        if token_data.claims.token_type != "access" {
            return Err(Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken));
        }

        Ok(token_data)
    }

    pub fn validate_refresh_token(&self, token: &str) -> Result<TokenData<RefreshClaims>, Error> {
        let token_data =
            decode::<RefreshClaims>(token, &self.decoding_key, &Validation::default())?;

        if token_data.claims.token_type != "refresh" {
            return Err(Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken));
        }

        Ok(token_data)
    }

    pub fn access_expiry_mins(&self) -> i64 {
        self.access_expiry_mins
    }

    pub fn refresh_expiry_days(&self) -> i64 {
        self.refresh_expiry_days
    }
}
