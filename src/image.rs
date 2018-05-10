use std::{
    error::Error,
    fmt,
    path::Path,
    borrow::Cow,
    marker::PhantomData,
    slice::from_raw_parts,
};
use super::*;
use format::*;
use symbol::*;
use symbolset::*;

pub type ZBarImageResult<'a> = Result<ZBarImage<'a>, ZBarImageError>;

#[derive(Debug)]
pub enum ZBarImageError {
    Len(u32, u32, usize),
}
impl Error for ZBarImageError {
    fn description(&self) -> &str { "image error" }
}
impl fmt::Display for ZBarImageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ZBarImageError::*;

        match *self {
            Len(w, h, l) => write!(f, "Width and height don't match actual data length => width: {}; height: {}; actual data length: {}", w, h, l)
        }
    }
}


pub struct ZBarImage<'a> {
    image: *mut zbar_image_s,
    userdata_len: Option<usize>,
    phantom: PhantomData<&'a ()>,
}
impl<'a> ZBarImage<'a> {
    unsafe fn from_raw(image: *mut zbar_image_s) -> Self {
        Self { image, userdata_len: None, phantom: PhantomData }
    }
    fn new<T>(width: u32,
              height: u32,
              format: &Format,
              data: T,
              cleanup_handler: Option<unsafe extern "C" fn (image: *mut zbar_image_t)>) -> ZBarImageResult<'a>

        where T: AsRef<[u8]>
    {
        let data = data.as_ref();
        match width as usize * height as usize == data.len() {
            true => unsafe {
                let image = zbar_image_create();
                zbar_image_set_format(image, format.fourcc().into());
                zbar_image_set_size(image, width, height);
                zbar_image_set_data(
                    image,
                    data.as_ptr() as *mut c_void,
                    (data.len() as u32).into(),
                    cleanup_handler
                );
                Ok(Self::from_raw(image))
            }
            false => Err(ZBarImageError::Len(width, height, data.len()))
        }

    }

    /// Creates a `ZBarImage` from owned data.
    ///
    /// # Examples
    ///
    /// ```
    /// use zbars::prelude::*;
    /// use std::borrow::Cow;
    ///
    /// // only data of size 1 for demonstration
    /// ZBarImage::from_owned(1, 1, &Format::from_label(Cow::Borrowed("Y8")), vec![1]).unwrap();
    ///
    /// ```
    pub fn from_owned(width: u32, height: u32, format: &Format, data: Vec<u8>) -> ZBarImageResult<'a> {
        let image = Self::new(width, height, format, data.as_slice(), Some(zbar_image_free_data))?;
        ::std::mem::forget(data);
        Ok(image)
    }

    /// Creates a `ZBarImage` from borrowed data.
    ///
    /// ```compile_fail
    /// use zbars::prelude::*;
    /// use std::borrow::Cow;
    ///
    /// create_image();
    ///
    /// fn create_image<'a>() -> ZBarImage<'a> {
    ///     ZBarImage::from_borrowed(1, 1, &Format::from_label(Cow::Borrowed("Y8")), &vec![1]).unwrap()
    /// }
    /// ```
    ///
    pub fn from_borrowed<T>(width: u32, height: u32, format: &Format, data: T) -> ZBarImageResult<'a>
        where T: AsRef<[u8]> + 'a
    {
        Self::new(width, height, format, data, None)
    }
    pub fn image_ref(&mut self) {
        //TODO: Needed?
        unimplemented!("TBD")
    }
    pub fn format(&self) -> Format {
        unsafe { Format::from_fourcc(zbar_image_get_format(**self) as u32) }
    }
    pub fn sequence(&self) -> u32 { unsafe { zbar_image_get_sequence(**self) } }
    pub fn width(&self) -> u32 { unsafe { zbar_image_get_width(**self) } }
    pub fn height(&self) -> u32 { unsafe { zbar_image_get_height(**self) } }
    pub fn data(&self) -> &[u8] {
        unsafe {
            from_raw_parts(
                zbar_image_get_data(**self) as *const u8,
                zbar_image_get_data_length(**self) as usize
            )
        }
    }
    pub fn symbols(&self) -> Option<SymbolSet> {
        SymbolSet::from_raw(unsafe { zbar_image_get_symbols(**self) })
    }
    pub fn set_symbols(&mut self, symbols: Option<&SymbolSet>) {
        unsafe { zbar_image_set_symbols(**self, symbols.map_or(::std::ptr::null(), |s| **s)) }
    }
    pub fn first_symbol(&self) -> Option<Symbol> {
        Symbol::from_raw(unsafe { zbar_image_first_symbol(self.image) })
    }
    pub fn set_sequence(&mut self, sequence_num: u32) {
        unsafe { zbar_image_set_sequence(**self, sequence_num) }
    }

    /// Just a crop with origin
    pub fn set_size(&mut self, width: u32, height: u32) {
        unsafe { zbar_image_set_size(**self, width, height) }
    }

    /// Sets userdata for `ZBarImage`.
    ///
    /// ```compile_fail
    /// use zbars::prelude::*;
    /// use std::borrow::Cow;
    ///
    /// let mut image =
    ///     ZBarImage::from_owned(1, 1, &Format::from_label(Cow::Borrowed("Y800")), vec![0]).unwrap();
    /// {
    ///     image.set_userdata(&vec![1]);
    /// }
    /// ```
    pub fn set_userdata<T>(&mut self, userdata: &'a T) where T: AsRef<[u8]> {
        let userdata = userdata.as_ref();
        self.userdata_len = Some(userdata.len());
        unsafe {
            zbar_image_set_userdata(
                **self,
                userdata.as_ref().as_ptr() as *mut u8 as *mut c_void)
        }
    }
    pub fn userdata(&self) -> Option<&'a [u8]>{
        self.userdata_len
            .map(|len| unsafe {
                from_raw_parts(zbar_image_get_userdata(**self) as *mut u8, len)
            })
    }
    pub fn write<P>(&self, path: P) -> ZBarResult<()> where P: AsRef<Path> {
        let result = unsafe {
            zbar_image_write(
                **self,
                path.as_ref().as_os_str().to_str().unwrap().as_bytes().as_ptr() as *mut i8,
            )
        };
        match result {
            0 => Ok(()),
            e => Err(e.into()),
        }
    }
    pub fn read<P>(_path: P) -> Option<Self> where P: AsRef<Path> {
        //TODO: zbar.h days: TBD
//        ZbarImage {
//            image: unsafe {
//                zbar_image_read(
//                    path.as_ref().as_os_str().to_str().unwrap().as_bytes().as_ptr() as *mut i8
//                )
//            }
//        }
        unimplemented!("zbar.h days: TBD")
    }
}

#[cfg(feature = "zbar_fork")]
impl<'a> ZBarImage<'a> {
    pub fn size(&self) -> (u32, u32) {
        unsafe {
            let mut dim = (0, 0);
            zbar_image_get_size(**self, &mut dim.0 as *mut u32, &mut dim.1 as *mut u32);
            dim
        }
    }
    pub fn crop(&self) -> (u32, u32, u32, u32) {
        unsafe {
            let mut crop = (0, 0, 0, 0);
            zbar_image_get_crop(
                **self,
                &mut crop.0 as *mut u32, &mut crop.1 as *mut u32,
                &mut crop.2 as *mut u32, &mut crop.3 as *mut u32
            );
            crop
        }
    }
    pub fn set_crop(&mut self, x: u32, y: u32, width: u32, height: u32) {
        unsafe { zbar_image_set_crop(**self, x, y, width, height) }
    }
}
impl<'a> Deref for ZBarImage<'a> {
    type Target = *mut zbar_image_s;
    fn deref(&self) -> &Self::Target { &self.image }
}
impl<'a> Drop for ZBarImage<'a> {
    fn drop(&mut self) { unsafe { zbar_image_destroy(**self) } }
}

#[cfg(feature = "from_image")]
pub mod from_image {

    extern crate image;

    use self::image::{
        GenericImage,
        DynamicImage,
        ImageResult,
        Pixel,
    };
    use super::*;

    lazy_static!(static ref FORMAT: Format<'static> = Format::from_label(Cow::Borrowed("Y800")););

    impl<'a> ZBarImage<'a> {
        /// Creates a `ZBarImage` from the given path.
        ///
        /// This method invokes `ZBarImage::from_dyn_image`. So if the image is already a Luma8
        /// no additional memory will be allocated.
        ///
        /// # Examples
        ///
        /// ```
        /// extern crate zbars;
        ///
        /// use zbars::image::ZBarImage;
        ///
        /// fn main() {
        ///     let image = ZBarImage::from_path("test/code128.gif").unwrap();
        /// }
        ///
        pub fn from_path<P>(path: P) -> ImageResult<Self> where P: AsRef<Path> {
            image::open(&path).map(Self::from_dyn_image)
        }

        /// Creates a `ZBarImage` from a `DynamicImage`.
        ///
        /// The given image will owned so zero copy takes place if the image is already a
        /// `DynamicImage::ImageLuma8`. If it is something other than Luma8 a new buffer will be
        /// allocated in order to grayscale the image.
        ///
        /// # Examples
        ///
        /// ```
        /// extern crate zbars;
        /// extern crate image;
        ///
        /// use zbars::image::ZBarImage;
        /// use image::{DynamicImage, ImageBuffer};
        ///
        /// fn main() {
        ///     let image = ZBarImage::from_dyn_image(
        ///         DynamicImage::ImageLuma8(
        ///             // small buffer just for demonstration
        ///             ImageBuffer::from_vec(1, 1, vec![0]).unwrap()
        ///         )
        ///     );
        /// }
        /// ```
        ///
        pub fn from_dyn_image(image: DynamicImage) -> Self {
            ZBarImage::from_owned(
                image.dimensions().0,
                image.dimensions().1,
                &FORMAT,
                match image {
                    DynamicImage::ImageLuma8(image) => image.into_raw(),
                    other                           => other.to_luma().into_raw(),
                })
                // Safe to unwrap here
                .unwrap()
        }

        /// Creates a `ZBarImage` from a `GenericImage`.
        ///
        /// As the pixel representation is not known for a `GenericImage` it will always
        /// be grayscaled and thus a new image buffer will be allocated. If possible use
        /// `ZBarImage::from_dyn_image` instead. Use this if you want to use `GenericImage`
        /// beyond this.
        ///
        /// # Examples
        ///
        /// ```
        /// extern crate zbars;
        /// extern crate image;
        ///
        /// use zbars::image::ZBarImage;
        /// use image::{DynamicImage, ImageBuffer};
        ///
        /// fn main() {
        ///     let image = ZBarImage::from_generic_image(
        ///         &DynamicImage::ImageRgb8(
        ///             // small buffer just for demonstration
        ///             ImageBuffer::from_vec(1, 1, vec![0, 0, 0]).unwrap()
        ///         )
        ///     );
        /// }
        /// ```
        ///
        pub fn from_generic_image<I>(image: &I) -> Self
            where I: GenericImage + 'static,
                  Vec<u8>: From<Vec<<<I as GenericImage>::Pixel as Pixel>::Subpixel>>
        {
            ZBarImage::from_owned(
                image.dimensions().0,
                image.dimensions().1,
                &FORMAT,
                image::imageops::grayscale(image).into_raw().into())
                // Safe to unwrap here
                .unwrap()
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_from_path() { assert!(ZBarImage::from_path("test/code128.gif").is_ok()); }

        #[test]
        fn test_from_dyn_image_luma() {
            use self::image::ImageBuffer;

            let data = vec![0, 0, 0];
            let image = ZBarImage::from_dyn_image(
                DynamicImage::ImageLuma8(ImageBuffer::from_vec(1, 3, data).unwrap())
            );
            assert_eq!(image.data(), &[0, 0, 0]);
        }

        #[test]
        fn test_from_dyn_image_rgb() {
            use self::image::ImageBuffer;

            let data = vec![0, 0, 0];
            let image = ZBarImage::from_dyn_image(
                DynamicImage::ImageRgb8(ImageBuffer::from_vec(1, 1, data).unwrap())
            );
            assert_eq!(image.data(), &[0]);
        }

        #[test]
        fn test_from_generic_image_luma() {
            use self::image::ImageBuffer;

            let data = vec![0, 0, 0];
            let image = ZBarImage::from_generic_image(
                &DynamicImage::ImageLuma8(ImageBuffer::from_vec(1, 3, data).unwrap())
            );
            assert_eq!(image.data(), &[0, 0, 0]);
        }

        #[test]
        fn test_from_generic_image_rgb() {
            use self::image::ImageBuffer;

            let data = vec![0, 0, 0];
            let image = ZBarImage::from_generic_image(
                &DynamicImage::ImageRgb8(ImageBuffer::from_vec(1, 1, data).unwrap())
            );
            assert_eq!(image.data(), &[0]);
        }
    }
}

#[cfg(test)]
#[cfg(feature = "zbar_fork")]
mod test_zbar_fork {
    use super::*;

    #[test]
    fn test_size() {
        assert_eq!(
            ZBarImage::from_owned(
                2, 3, &Format::from_label(Cow::Borrowed("Y800")), [0; 2 * 3].to_vec()
            ).unwrap().size(),
            (2, 3)
        );
    }

    #[test]
    fn test_crop() {
        let mut image = ZBarImage::from_owned(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), [0; 20 * 30].to_vec()
        ).unwrap();
        assert_eq!(image.crop(), (0, 0, 20, 30));
    }

    #[test]
    fn test_set_crop_smaller() {
        let mut image = ZBarImage::from_owned(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), [0; 20 * 30].to_vec()
        ).unwrap();
        image.set_crop(5, 5, 10, 10);
        assert_eq!(image.crop(), (5, 5, 10, 10));
    }

    #[test]
    fn test_set_crop_larger() {
        let mut image = ZBarImage::from_owned(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), [0; 20 * 30].to_vec()
        ).unwrap();
        image.set_crop(5, 50, 100, 200);
        assert_eq!(image.crop(), (5, 30, 15, 0));
    }
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod test_mem {

    extern crate procinfo;

    use super::*;

    const N: usize = 100000;

    #[test]
    fn test_mem_from_buf() {
        let mem_before = mem();
        for _ in 0..N {
            let buf = [0; 500 * 500];
            ZBarImage::from_owned(
                500, 500, &Format::from_label(Cow::Borrowed("Y800")), buf.to_vec()
            ).unwrap();
        }
        assert_mem(mem_before, N);
    }

    #[test]
    fn test_mem_from_slice() {
        let mem_before = mem();
        for _ in 0..N {
            let buf = [0; 500 * 500];
            ZBarImage::from_borrowed(
                500, 500, &Format::from_label(Cow::Borrowed("Y800")), buf.as_ref()
            ).unwrap();
        }
        assert_mem(mem_before, N);
    }

    fn mem() -> usize { procinfo::pid::statm_self().unwrap().resident }

    fn assert_mem(mem_before: usize, n: usize) {
        let mem_after = mem();
        // Allow memory to grow by 8MB, but not more.
        assert!(
            mem_after < mem_before + 8 * 1024,
            "Memory usage at start is {}KB, memory usage after {} loops is {}KB",
            mem_before, n, mem_after
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn format() {
        let format = Format::from_label(Cow::Borrowed("Y800"));
        assert_eq!(
            ZBarImage::from_owned(
                2, 3, &format, [0; 2 * 3].to_vec()
            ).unwrap().format(),
            format
        );
    }

    #[test]
    fn test_sequence_set_and_get() {
        let mut image = ZBarImage::from_owned(
            2, 3, &Format::from_label(Cow::Borrowed("Y800")), [0; 2 * 3].to_vec()
        ).unwrap();
        assert_eq!(image.sequence(), 0);
        image.set_sequence(1);
        assert_eq!(image.sequence(), 1);
        image.set_sequence(999);
        assert_eq!(image.sequence(), 999);
    }

    #[test]
    fn test_set_size_smaller() {
        let mut image = ZBarImage::from_owned(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), [0; 20 * 30].to_vec()
        ).unwrap();
        image.set_size(10, 12);
        assert_eq!(image.width(), 10);
        assert_eq!(image.height(), 12);
    }

    #[test]
    fn test_set_size_larger() {
        let mut image = ZBarImage::from_owned(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), [0; 20 * 30].to_vec()
        ).unwrap();
        image.set_size(100, 120);
        assert_eq!(image.width(), 100);
        assert_eq!(image.height(), 120);
    }

    #[test]
    fn test_width() {
        assert_eq!(
            ZBarImage::from_owned(
                2, 3, &Format::from_label(Cow::Borrowed("Y800")), [0; 2 * 3].to_vec()
            ).unwrap().width(),
            2
        );
    }

    #[test]
    fn test_height() {
        assert_eq!(
            ZBarImage::from_owned(
                2, 3, &Format::from_label(Cow::Borrowed("Y800")), [0; 2 * 3].to_vec()
            ).unwrap().height(),
            3
        );
    }

    #[test]
    fn test_data() {
        let buf = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let image = ZBarImage::from_owned(
            3, 4, &Format::from_label(Cow::Borrowed("Y800")), buf.clone()
        ).unwrap();
        assert_eq!(buf.as_slice(), image.data());
    }

    #[test]
    fn test_symbols_get_and_set() {
        let mut image = ZBarImage::from_owned(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), [0; 20 * 30].to_vec()
        ).unwrap();
        assert!(image.symbols().is_none());
        image.set_symbols(None);
        assert!(image.symbols().is_none());
    }

    #[test]
    fn test_first_symbol() {
        assert!(
            ZBarImage::from_owned(
                20, 30, &Format::from_label(Cow::Borrowed("Y800")), [0; 20 * 30].to_vec()
            ).unwrap().first_symbol().is_none()
        );
    }

    #[test]
    fn test_userdata_set_and_get() {
        let mut userdata = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let data = vec![0; 20 * 30];

        let mut image1 = ZBarImage::from_owned(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), data.clone()
        ).unwrap();
        let mut image2 = ZBarImage::from_borrowed(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), data.as_slice()
        ).unwrap();
        let mut image3 = ZBarImage::from_borrowed(
            20, 30, &Format::from_label(Cow::Borrowed("Y800")), data.as_slice()
        ).unwrap();

        assert!(image1.userdata().is_none());

        image1.set_userdata(&userdata);
        image2.set_userdata(&userdata);
        image3.set_userdata(&userdata);

        assert!(image1.userdata().is_some());
        assert_eq!(image1.userdata(), image2.userdata());
        assert_eq!(image1.userdata(), image3.userdata());
    }

    #[test]
    fn test_write() {
        let path = std::env::temp_dir().join("zbar_image");
        let image = ZBarImage::from_owned(
            2, 3, &Format::from_label(Cow::Borrowed("Y800")), [0; 2 * 3].to_vec()
        ).unwrap();
        assert!(image.write(&path).is_ok());
    }

    #[test]
    fn test_write_fail() {
        let path = Path::new("/nowhere/nothing");
        let image = ZBarImage::from_owned(
            2, 3, &Format::from_label(Cow::Borrowed("Y800")), [0; 2 * 3].to_vec()
        ).unwrap();
        assert!(image.write(&path).is_err());
    }
}
