use bstr::{BString, ByteSlice};
use git2::{ObjectType, Repository};

pub struct Status {
    pub head: HeadStatus,
}

pub struct HeadStatus {
    pub name: BString,
    pub detached: bool,
}

pub fn get_status(repo: &mut Repository) -> Result<Status, git2::Error> {
    Ok(Status {
        head: get_head_status(repo)?,
    })
}

fn get_head_status(repo: &mut Repository) -> Result<HeadStatus, git2::Error> {
    let detached = repo.head_detached()?;
    let name = match repo.head() {
        Ok(head) => {
            if detached {
                let object = head.peel(ObjectType::Any)?;

                let describe_result = object.describe(
                    &git2::DescribeOptions::new()
                        .describe_tags()
                        .max_candidates_tags(1),
                );
                if let Ok(description) = describe_result {
                    description.format(None)?.into()
                } else {
                    object
                        .short_id()?
                        .as_str()
                        .expect("oid is invalid utf-8")
                        .into()
                }
            } else {
                head.shorthand_bytes().as_bstr().to_owned()
            }
        }
        Err(err) if err.code() == git2::ErrorCode::UnbornBranch => "master".into(),
        Err(err) => return Err(err),
    };

    Ok(HeadStatus { name, detached })
}
