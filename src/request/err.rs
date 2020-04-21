use std::fmt;
#[derive(Debug)]
pub enum RequestErr {
    PgPool(r2d2::Error),
    Pg(postgres::Error),
}

impl std::error::Error for RequestErr {}

impl fmt::Display for RequestErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use RequestErr::*;
        let msg = match self {
            PgPool(e) => format!("{}", e),
            Pg(e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl From<r2d2::Error> for RequestErr {
    fn from(e: r2d2::Error) -> Self {
        Self::PgPool(e)
    }
}
impl From<postgres::Error> for RequestErr {
    fn from(e: postgres::Error) -> Self {
        Self::Pg(e)
    }
}
