use nix::unistd::Uid;

pub(crate) fn is_user_admin() -> bool {
    Uid::effective().is_root()
}
