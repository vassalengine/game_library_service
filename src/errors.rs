#[derive(Debug)]
pub enum AppError {
    InternalError,
    Unauthorized,
    DatabaseError(String),
    NotImplemented
}