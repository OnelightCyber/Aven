//! A picture of the screen the browser is on, for the person to hand to the
//! assistant. It shows whatever is on that screen, other applications
//! included, so it is only ever taken because the person attached it to a
//! message, once per message, and it is never kept.

/// The widest the picture is drawn and the most it may weigh: enough to read
/// a screen, and within what the assistant admits.
const MAX_WIDTH: u32 = 1_920;
const MAX_BYTES: usize = 4 * 1024 * 1024;

/// The screen under the window in front, as JPEG bytes. None when it cannot
/// be taken.
#[cfg(not(target_os = "windows"))]
pub(crate) fn capture() -> Option<Vec<u8>> {
    None
}

#[cfg(target_os = "windows")]
pub(crate) fn capture() -> Option<Vec<u8>> {
    use windows::Win32::{Graphics::Gdi::*, UI::WindowsAndMessaging::*};

    // SAFETY: plain queries and GDI objects created, used and released here.
    // The pixel buffer belongs to the DIB section and is read before the
    // section is deleted.
    unsafe {
        let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let area = info.rcMonitor;
        let (width, height) = (area.right - area.left, area.bottom - area.top);
        // Bound the allocation whatever the system reports.
        if !(1..=16_384).contains(&width) || !(1..=16_384).contains(&height) {
            return None;
        }
        let header = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(None, &header, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        let memory = CreateCompatibleDC(None);
        let screen = GetDC(None);
        let mut picture = None;
        if !memory.0.is_null() && !screen.0.is_null() && !bits.is_null() {
            let previous = SelectObject(memory, bitmap.into());
            let copied = BitBlt(
                memory,
                0,
                0,
                width,
                height,
                Some(screen),
                area.left,
                area.top,
                SRCCOPY | CAPTUREBLT,
            )
            .is_ok();
            SelectObject(memory, previous);
            if copied {
                let pixels = std::slice::from_raw_parts(
                    bits.cast::<u8>(),
                    width as usize * height as usize * 4,
                );
                picture = encode(width as u32, height as u32, pixels);
            }
        }
        if !screen.0.is_null() {
            ReleaseDC(None, screen);
        }
        if !memory.0.is_null() {
            let _ = DeleteDC(memory);
        }
        let _ = DeleteObject(bitmap.into());
        picture
    }
}

/// Blue-green-red-unused pixels, top row first, as a JPEG no wider than the
/// limit.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn encode(width: u32, height: u32, pixels: &[u8]) -> Option<Vec<u8>> {
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for pixel in pixels.chunks_exact(4) {
        rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
    }
    let mut picture = image::RgbImage::from_raw(width, height, rgb)?;
    if width > MAX_WIDTH {
        let scaled = (u64::from(height) * u64::from(MAX_WIDTH) / u64::from(width)).max(1) as u32;
        picture = image::imageops::resize(
            &picture,
            MAX_WIDTH,
            scaled,
            image::imageops::FilterType::Triangle,
        );
    }
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 78)
        .encode_image(&picture)
        .ok()?;
    (out.len() <= MAX_BYTES).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_become_a_bounded_jpeg_with_their_colours_the_right_way_round() {
        // Two by one: a blue pixel, then a red one, as Windows lays them out.
        let jpeg = encode(2, 1, &[255, 0, 0, 0, 0, 0, 255, 0]).expect("a picture");
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
        let back = image::load_from_memory(&jpeg).expect("decodes").to_rgb8();
        assert_eq!(back.dimensions(), (2, 1));
        assert!(
            back.get_pixel(0, 0)[2] > back.get_pixel(0, 0)[0],
            "the first pixel is blue"
        );
        assert!(
            back.get_pixel(1, 0)[0] > back.get_pixel(1, 0)[2],
            "the second is red"
        );
    }

    #[test]
    fn a_wide_screen_is_drawn_no_wider_than_the_limit() {
        let (width, height) = (MAX_WIDTH * 2, 8);
        let pixels = vec![128u8; width as usize * height as usize * 4];
        let jpeg = encode(width, height, &pixels).expect("a picture");
        let back = image::load_from_memory(&jpeg).expect("decodes");
        assert_eq!((back.width(), back.height()), (MAX_WIDTH, 4));
    }

    #[test]
    fn a_buffer_of_the_wrong_size_gives_nothing() {
        assert!(encode(4, 4, &[0u8; 8]).is_none());
    }
}
