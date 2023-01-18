/// Optional medata desctibing:
/// Author of the model, organization that created the model, save date(unix timestamp), is file fully adhering to the standard or is it using any extensions. It also describes which minimal version is needed to open it. This **does not** mean that it contains some data or segment that is not supported, this is reserved for **breaking changes**. This has default value of 0 0(any verson is supported)
pub struct FileMetadata{
    author:String,
	org:String,
	save_date:u64,
	is_standard:bool,
	min_major:u16,
	min_minor:u16,
}
impl FileMetadata{
	/// Empty, defaut metadata, describing model with no known author, no organication, created on midnight 1.1.1970, and using no extensions.
	pub fn empty()->Self{
		Self{author:"".to_owned(),org:"".to_owned(),save_date:0,is_standard:true,min_major:0,min_minor:0}
	}
}
