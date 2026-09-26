use crate::models::user::User;
use crate::repository::users_repo::UsersRepository;
use uuid::Uuid;

pub struct UsersService {
    repo: UsersRepository,
}

impl UsersService {
    pub fn new(repo: UsersRepository) -> Self {
        Self { repo }
    }

    pub async fn get_by_id(&self, id: &Uuid) -> anyhow::Result<Option<User>> {
        let user = self.repo.get_by_id(id).await?;
        Ok(user)
    }
}
