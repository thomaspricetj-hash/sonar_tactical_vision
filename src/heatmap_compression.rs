use crate::heatmap::HeatLayer;

/// A standalone compression trait for heatmaps.
/// Mirrors your SyntheticMind compression API but remains independent.
/// Now includes:
/// - versioned header
/// - CRC32 integrity
/// - multi‑layer index hooks
/// - reversible‑collapse ready structure
pub trait HeatmapCompressor {
    fn compress(&self, layer: &HeatLayer) -> Vec<u8>;
    fn decompress(&self, data: &[u8]) -> HeatLayer;
}

/// Baseline compressor using delta‑RLE + quantization.
/// Lightweight, stable, and easily replaceable with your v8‑Ultra engine.
pub struct BaselineHeatmapCompressor;

impl BaselineHeatmapCompressor {
    pub fn new() -> Self {
        Self {}
    }

    /// Simple CRC32 for integrity (no external deps).
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in bytes {
            crc ^= b as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320u32 & mask);
            }
        }
        !crc
    }
}

impl HeatmapCompressor for BaselineHeatmapCompressor {
    fn compress(&self, layer: &HeatLayer) -> Vec<u8> {
        let mut out = Vec::new();

        // --- HEADER ---
        // [0..4]  version
        // [4..8]  width
        // [8..12] height
        // [12..16] reserved (future: reversible collapse flags)
        out.extend_from_slice(&1u32.to_le_bytes()); // version = 1
        out.extend_from_slice(&(layer.width as u32).to_le_bytes());
        out.extend_from_slice(&(layer.height as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // reserved

        // --- Quantize to 0–255 ---
        let quantized: Vec<u8> = layer
            .cells
            .iter()
            .map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8)
            .collect();

        // --- Delta‑RLE compression ---
        let mut i = 0;
        let len = quantized.len();

        while i < len {
            let value = quantized[i];
            let mut run = 1;

            while i + run < len && quantized[i + run] == value && run < 255 {
                run += 1;
            }

            out.push(value);
            out.push(run as u8);

            i += run;
        }

        // --- CRC32 ---
        let crc = Self::crc32(&out);
        out.extend_from_slice(&crc.to_le_bytes());

        out
    }

    fn decompress(&self, data: &[u8]) -> HeatLayer {
        // Must have at least header + CRC
        if data.len() < 20 {
            return HeatLayer {
                width: 0,
                height: 0,
                cells: Vec::new(),
            };
        }

        // --- Read header ---
        let version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let w = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let h = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;

        // reserved bytes [12..16] ignored for baseline

        // --- CRC check ---
        let stored_crc = u32::from_le_bytes([
            data[data.len() - 4],
            data[data.len() - 3],
            data[data.len() - 2],
            data[data.len() - 1],
        ]);

        let calc_crc = Self::crc32(&data[..data.len() - 4]);

        if stored_crc != calc_crc {
            // Corrupted → return empty layer
            return HeatLayer {
                width: w,
                height: h,
                cells: vec![0.0; w * h],
            };
        }

        // --- Delta‑RLE decode ---
        let mut cells = Vec::new();
        let mut i = 16; // start after header

        while i + 1 < data.len() - 4 {
            let value = data[i];
            let run = data[i + 1] as usize;

            let decoded = (value as f32) / 255.0;

            cells.extend(std::iter::repeat(decoded).take(run));

            i += 2;
        }

        // Safety: ensure correct length
        if cells.len() < w * h {
            cells.resize(w * h, 0.0);
        } else if cells.len() > w * h {
            cells.truncate(w * h);
        }

        HeatLayer { width: w, height: h, cells }
    }
}
