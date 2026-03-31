use crate::error::AppError;
use validator::Validate;

pub fn validate<T: Validate>(data: &T) -> Result<(), AppError> {
    data.validate()?;
    Ok(())
}
