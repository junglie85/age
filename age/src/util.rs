pub fn cast_slice<T: Copy>(s: &[T]) -> &[u8] {
    let len = std::mem::size_of_val(s);
    let data = s.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(data, len) }
}
