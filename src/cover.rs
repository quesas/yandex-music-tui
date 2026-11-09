/// Небольшая обложка, уже приведённая к RGB и уменьшенная для терминала.
#[derive(Debug, Clone)]
pub struct CoverImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 3]>,
}

impl CoverImage {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let image = image::load_from_memory(bytes)
            .map_err(|e| format!("не удалось декодировать обложку: {e}"))?
            .thumbnail(192, 192);
        let image = image.to_rgb8();
        let (width, height) = image.dimensions();
        if width == 0 || height == 0 {
            return Err("обложка пустая".into());
        }

        Ok(Self {
            width,
            height,
            pixels: image.pixels().map(|pixel| pixel.0).collect(),
        })
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));
        self.pixels[(y * self.width + x) as usize]
    }
}
