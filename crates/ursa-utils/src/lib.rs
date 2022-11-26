/// convert between different types of cids e.g libapld::Cid to cid::Cid
pub fn convert_cid<T>(cid: Vec<u8>) -> T
where
    T: TryFrom<Vec<u8>>,
    <T as TryFrom<Vec<u8>>>::Error: std::fmt::Debug,
{
    T::try_from(cid).unwrap()
}
