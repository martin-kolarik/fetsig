#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Undefined = 900,

    FetchFailed = 901,
    FetchTimeout = 902,
    DecodeFailed = 903,

    Ok = 200,
    Created = 201,
    NoContent = 204,

    NotModified = 304,

    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    Conflict = 409,
    PayloadTooBig = 413,
    UnsupportedMediaType = 415,
    RateLimited = 429,

    InternalServerError = 500,
    NotImplemented = 501,
}

impl StatusCode {
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            Self::Ok | Self::Created | Self::NoContent | Self::NotModified
        )
    }

    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::FetchFailed | Self::FetchTimeout)
    }
}

impl From<bool> for StatusCode {
    fn from(success: bool) -> Self {
        if success {
            StatusCode::Ok
        } else {
            StatusCode::BadRequest
        }
    }
}

impl From<u16> for StatusCode {
    fn from(code: u16) -> Self {
        match code {
            200 => Self::Ok,
            201 => Self::Created,
            204 => Self::NoContent,
            304 => Self::NotModified,
            400 => Self::BadRequest,
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            405 => Self::MethodNotAllowed,
            409 => Self::Conflict,
            413 => Self::PayloadTooBig,
            415 => Self::UnsupportedMediaType,
            429 => Self::RateLimited,
            500 => Self::InternalServerError,
            501 => Self::NotImplemented,
            901 => Self::FetchFailed,
            902 => Self::FetchTimeout,
            903 => Self::DecodeFailed,
            _ => Self::Undefined,
        }
    }
}
