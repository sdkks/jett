use ::std::ffi::OsString;

use crate::state::files::{FileOrFolder, Folder};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileType {
    File,
    Folder,
}

#[derive(Debug, Clone)]
pub struct ChildPreview {
    pub name: OsString,
    pub size: u128,
    pub file_type: FileType,
}

#[derive(Debug, Clone)]
pub struct FolderComposition {
    pub direct_count: u64,
    pub top_children: [Option<ChildPreview>; 3],
}

impl From<&Folder> for FolderComposition {
    fn from(folder: &Folder) -> Self {
        // Keep borrowed candidates until the ranking is final: only three names are cloned.
        let mut top: [Option<(&OsString, &FileOrFolder)>; 3] = [None; 3];
        for (name, child) in &folder.contents {
            let position = top.iter().position(|slot| {
                slot.is_none_or(|(other_name, other)| {
                    child.size() > other.size()
                        || (child.size() == other.size() && name < other_name)
                })
            });
            if let Some(position) = position {
                for i in (position + 1..3).rev() {
                    top[i] = top[i - 1];
                }
                top[position] = Some((name, child));
            }
        }
        Self {
            direct_count: folder.contents.len() as u64,
            top_children: top.map(|slot| {
                slot.map(|(name, child)| ChildPreview {
                    name: name.clone(),
                    size: child.size(),
                    file_type: match child {
                        FileOrFolder::Folder(_) => FileType::Folder,
                        FileOrFolder::File(_) => FileType::File,
                    },
                })
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub name: OsString,
    pub size: u128,
    pub descendants: Option<u64>,
    pub percentage: f64, // 1.0 is 100% (0.5 is 50%, etc.)
    pub file_type: FileType,
    pub composition: Option<FolderComposition>,
}

fn calculate_percentage(size: u128, total_size: u128, total_files_in_parent: usize) -> f64 {
    if size == 0 && total_size == 0 {
        // if all files in the folder are of size 0, we'll want to display them all as
        // the same size
        1.0 / total_files_in_parent as f64
    } else {
        size as f64 / total_size as f64
    }
}

pub fn files_in_folder(folder: &Folder, offset: usize) -> Vec<FileMetadata> {
    let mut files = Vec::new();
    let total_size = folder.size;
    for (name, file_or_folder) in &folder.contents {
        files.push({
            let size = file_or_folder.size();
            let name = name.clone();
            let (descendants, file_type, composition) = match file_or_folder {
                FileOrFolder::Folder(folder) => (
                    Some(folder.num_descendants),
                    FileType::Folder,
                    Some(FolderComposition::from(folder)),
                ),
                FileOrFolder::File(_file) => (None, FileType::File, None),
            };
            let percentage = calculate_percentage(size, total_size, folder.contents.len());
            FileMetadata {
                size,
                name,
                descendants,
                percentage,
                file_type,
                composition,
            }
        });
    }
    files.sort_by(|a, b| {
        if a.percentage == b.percentage {
            a.name.partial_cmp(&b.name).expect("could not compare name")
        } else {
            b.percentage
                .partial_cmp(&a.percentage)
                .expect("could not compare percentage")
        }
    });
    if offset > 0 {
        let removed_items = files.drain(..offset);
        let number_of_files_without_removed_contents = folder.contents.len() - removed_items.len();
        let removed_size = removed_items.fold(0, |acc, file| acc + file.size);
        let size_without_removed_items = total_size - removed_size;
        for file in &mut files {
            file.percentage = calculate_percentage(
                file.size,
                size_without_removed_items,
                number_of_files_without_removed_contents,
            );
        }
    }
    files
}
