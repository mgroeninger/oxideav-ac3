//! Custom Channel Map (Table E2.5 — `chanmap` field) decoder.
//!
//! The 16-bit `chanmap` field is emitted on dependent substreams when
//! `chanmape == 1` (§E.2.3.1.7-8). It assigns each coded channel of
//! the dep substream to a fixed channel location drawn from a 16-slot
//! reference grid (Table E2.5). The MSB of the field is bit 0
//! ("Left") and the LSB is bit 15 ("LFE").
//!
//! Per spec (§E.2.3.1.8):
//!
//! > Bit 0, which indicates the presence of the left channel, is
//! > stored in the most significant bit of the chanmap field. For
//! > each channel present in the dependent substream, the
//! > corresponding location bit in the chanmap is set to '1'. The
//! > order of the coded channels in the dependent substream is the
//! > same as the order of the enabled location bits in the chanmap.
//! > […] When the enabled location bit in the chanmap field refers
//! > to a pair of channels, this defines the channel location of two
//! > adjacent channels in the dependent substream.
//!
//! The pair-bits (Table E2.5 entries that expand to **two** adjacent
//! channels) are bits 5, 6, 9, 10, 11, and 13.
//!
//! The spec constraint "the number of channel locations indicated by
//! the chanmap field must equal the total number of coded channels
//! present in the dependent substream, as indicated by the acmod and
//! lfeon bit stream parameters" is enforced by
//! [`expand_chanmap_locations`] returning [`ChanmapError::CountMismatch`]
//! when the expanded count does not match `dep_nchans`.
//!
//! This module is consumed by [`crate::eac3::decoder`] when splicing a
//! dependent substream's PCM into the independent substream's program.

/// One physical channel-location slot per Table E2.5.
///
/// The numeric value is the bit index in the 16-bit `chanmap` field
/// (NOT the bit weight); pair-bits expand to two distinct enum
/// variants in the order specified by the spec text ("first coded
/// channel is the Left Surround channel, the second coded channel
/// is the Right Surround channel"). For pair bit 6 ("Lrs/Rrs pair")
/// this yields [`ChannelLocation::LeftRearSurround`] then
/// [`ChannelLocation::RightRearSurround`]; for bit 9 ("Lsd/Rsd
/// pair") this yields [`ChannelLocation::LeftSurroundDirect`] then
/// [`ChannelLocation::RightSurroundDirect`]; etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChannelLocation {
    /// Bit 0 — Left.
    Left,
    /// Bit 1 — Center.
    Center,
    /// Bit 2 — Right.
    Right,
    /// Bit 3 — Left Surround.
    LeftSurround,
    /// Bit 4 — Right Surround.
    RightSurround,
    /// Bit 5 (pair) — left half of Lc/Rc.
    LeftCenter,
    /// Bit 5 (pair) — right half of Lc/Rc.
    RightCenter,
    /// Bit 6 (pair) — left half of Lrs/Rrs (left rear surround).
    LeftRearSurround,
    /// Bit 6 (pair) — right half of Lrs/Rrs (right rear surround).
    RightRearSurround,
    /// Bit 7 — Cs (center surround).
    CenterSurround,
    /// Bit 8 — Ts (top surround).
    TopSurround,
    /// Bit 9 (pair) — left half of Lsd/Rsd (left surround direct).
    LeftSurroundDirect,
    /// Bit 9 (pair) — right half of Lsd/Rsd (right surround direct).
    RightSurroundDirect,
    /// Bit 10 (pair) — left half of Lw/Rw.
    LeftWide,
    /// Bit 10 (pair) — right half of Lw/Rw.
    RightWide,
    /// Bit 11 (pair) — left half of Vhl/Vhr (vertical-height left).
    VerticalHeightLeft,
    /// Bit 11 (pair) — right half of Vhl/Vhr.
    VerticalHeightRight,
    /// Bit 12 — Vhc (vertical-height center).
    VerticalHeightCenter,
    /// Bit 13 (pair) — left half of Lts/Rts (top surround left).
    TopSurroundLeft,
    /// Bit 13 (pair) — right half of Lts/Rts.
    TopSurroundRight,
    /// Bit 14 — LFE2 (second low-frequency effect).
    Lfe2,
    /// Bit 15 — LFE.
    Lfe,
}

/// Errors raised by the chanmap decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChanmapError {
    /// The expanded chanmap location count does not match the dep
    /// substream's coded channel count. Per §E.2.3.1.8 this is a
    /// bit-stream violation.
    CountMismatch {
        expanded: u8,
        dep_nchans: u8,
        chanmap: u16,
    },
}

impl core::fmt::Display for ChanmapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChanmapError::CountMismatch {
                expanded,
                dep_nchans,
                chanmap,
            } => write!(
                f,
                "eac3 chanmap: expanded {} locations but dep substream codes {} channels (chanmap=0x{:04X})",
                expanded, dep_nchans, chanmap
            ),
        }
    }
}

impl core::error::Error for ChanmapError {}

/// Expand the 16-bit `chanmap` field into the ordered list of channel
/// locations carried by the dep substream.
///
/// `dep_nchans` is the dependent substream's total coded channel
/// count (`acmod_nfchans(acmod) + lfeon as u8`); the function checks
/// the spec invariant that the expanded count equals `dep_nchans`.
///
/// Iteration order is bit 0 (Left) → bit 15 (LFE), i.e. MSB→LSB of
/// the `chanmap` field. Pair-bits produce two consecutive entries in
/// the order documented on each variant.
pub fn expand_chanmap_locations(
    chanmap: u16,
    dep_nchans: u8,
) -> Result<Vec<ChannelLocation>, ChanmapError> {
    let mut out: Vec<ChannelLocation> = Vec::with_capacity(16);
    // Iterate bit indices 0..16. Bit 0 sits in the MSB of the 16-bit
    // field, so it has weight `1 << 15`.
    for bit in 0u8..16 {
        let mask = 1u16 << (15 - bit);
        if chanmap & mask == 0 {
            continue;
        }
        let push_results: &[ChannelLocation] = match bit {
            0 => &[ChannelLocation::Left],
            1 => &[ChannelLocation::Center],
            2 => &[ChannelLocation::Right],
            3 => &[ChannelLocation::LeftSurround],
            4 => &[ChannelLocation::RightSurround],
            5 => &[ChannelLocation::LeftCenter, ChannelLocation::RightCenter],
            6 => &[
                ChannelLocation::LeftRearSurround,
                ChannelLocation::RightRearSurround,
            ],
            7 => &[ChannelLocation::CenterSurround],
            8 => &[ChannelLocation::TopSurround],
            9 => &[
                ChannelLocation::LeftSurroundDirect,
                ChannelLocation::RightSurroundDirect,
            ],
            10 => &[ChannelLocation::LeftWide, ChannelLocation::RightWide],
            11 => &[
                ChannelLocation::VerticalHeightLeft,
                ChannelLocation::VerticalHeightRight,
            ],
            12 => &[ChannelLocation::VerticalHeightCenter],
            13 => &[
                ChannelLocation::TopSurroundLeft,
                ChannelLocation::TopSurroundRight,
            ],
            14 => &[ChannelLocation::Lfe2],
            15 => &[ChannelLocation::Lfe],
            _ => unreachable!(),
        };
        for &loc in push_results {
            // At most 16 distinct entries (pair-bits use 2 slots each;
            // total expanded count cannot exceed 16 since the spec
            // limits dep-substream coded channels to A/52's maximum).
            out.push(loc);
        }
    }

    let expanded = out.len() as u8;
    if expanded != dep_nchans {
        return Err(ChanmapError::CountMismatch {
            expanded,
            dep_nchans,
            chanmap,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec example #1 (§E.2.3.1.8): "if bits 0, 3, and 4 of the
    /// chanmap field are set to '1', and the dependent stream is
    /// coded with acmod = 3 and lfeon = 0, the first coded channel
    /// in the dependent stream is the Left channel, the second
    /// coded channel is the Left Surround channel, and the third
    /// coded channel is the Right Surround channel."
    #[test]
    fn spec_example_bits_0_3_4() {
        // Bit 0 = MSB = 0x8000; bit 3 = 0x1000; bit 4 = 0x0800.
        let chanmap = 0x8000 | 0x1000 | 0x0800;
        let dep_nchans = 3; // acmod=3 (3-ch) + lfeon=0
        let locs = expand_chanmap_locations(chanmap, dep_nchans).unwrap();
        assert_eq!(locs.len(), 3);
        assert_eq!(locs[0], ChannelLocation::Left);
        assert_eq!(locs[1], ChannelLocation::LeftSurround);
        assert_eq!(locs[2], ChannelLocation::RightSurround);
    }

    /// Spec example #2 (§E.2.3.1.8): "if bits 3, 4 and 6 of the
    /// chanmap field are set to '1', and the dependent stream is
    /// coded with acmod = 6 and lfeon = '0', the first coded channel
    /// in the dependent stream is the Left Surround channel, the
    /// second coded channel is the Right Surround channel, and the
    /// third and fourth channels are the Left Rear Surround and
    /// Right Rear Surround channels."
    #[test]
    fn spec_example_bits_3_4_6_pair() {
        // Bit 3 = 0x1000; bit 4 = 0x0800; bit 6 (pair) = 0x0200.
        let chanmap = 0x1000 | 0x0800 | 0x0200;
        let dep_nchans = 4; // acmod=6 (4-ch) + lfeon=0
        let locs = expand_chanmap_locations(chanmap, dep_nchans).unwrap();
        assert_eq!(locs.len(), 4);
        assert_eq!(locs[0], ChannelLocation::LeftSurround);
        assert_eq!(locs[1], ChannelLocation::RightSurround);
        assert_eq!(locs[2], ChannelLocation::LeftRearSurround);
        assert_eq!(locs[3], ChannelLocation::RightRearSurround);
    }

    /// The in-tree E-AC-3 encoder emits 7.1 as indep 5.1 (acmod=7,
    /// lfeon=1) plus a dep substream carrying the Lb/Rb pair with
    /// chanmap bit 6 ("Lrs/Rrs pair") set, dep acmod=2 (2 coded
    /// channels). Decoder must round-trip the pair as the two rear-
    /// surround channels.
    #[test]
    fn encoder_71_lb_rb_pair() {
        // Bit 6 weight = 1 << (15 - 6) = 0x0200.
        let chanmap = 0x0200;
        let dep_nchans = 2; // acmod=2 (2-ch) + lfeon=0
        let locs = expand_chanmap_locations(chanmap, dep_nchans).unwrap();
        assert_eq!(locs.len(), 2);
        assert_eq!(locs[0], ChannelLocation::LeftRearSurround);
        assert_eq!(locs[1], ChannelLocation::RightRearSurround);
    }

    /// Spec invariant: expanded count must equal dep_nchans. A pair
    /// bit set with dep_nchans=1 is a bit-stream violation.
    #[test]
    fn count_mismatch_rejected() {
        let chanmap = 0x0200; // bit 6 (pair) — expands to 2
        let dep_nchans = 1; // mismatched
        let err = expand_chanmap_locations(chanmap, dep_nchans).unwrap_err();
        assert!(matches!(err, ChanmapError::CountMismatch { .. }));
    }

    /// Bit 0 lives at MSB; bit 15 lives at LSB. Sanity-check the
    /// extreme bits and confirm the iteration order picks low-index
    /// (MSB) bits first.
    #[test]
    fn msb_lsb_extremes_and_order() {
        // Bits 0 (Left, MSB) + 15 (LFE, LSB).
        let chanmap = 0x8000 | 0x0001;
        let locs = expand_chanmap_locations(chanmap, 2).unwrap();
        assert_eq!(locs.len(), 2);
        assert_eq!(locs[0], ChannelLocation::Left);
        assert_eq!(locs[1], ChannelLocation::Lfe);
    }

    /// Every single-channel bit (i.e. non-pair bits) — exercises
    /// all 10 non-pair Table E2.5 rows.
    #[test]
    fn all_single_bits_decode() {
        let mut chanmap = 0u16;
        let single_bits = [0, 1, 2, 3, 4, 7, 8, 12, 14, 15];
        for &b in &single_bits {
            chanmap |= 1u16 << (15 - b);
        }
        let locs = expand_chanmap_locations(chanmap, single_bits.len() as u8).unwrap();
        assert_eq!(locs.len(), single_bits.len());
        let expected = [
            ChannelLocation::Left,
            ChannelLocation::Center,
            ChannelLocation::Right,
            ChannelLocation::LeftSurround,
            ChannelLocation::RightSurround,
            ChannelLocation::CenterSurround,
            ChannelLocation::TopSurround,
            ChannelLocation::VerticalHeightCenter,
            ChannelLocation::Lfe2,
            ChannelLocation::Lfe,
        ];
        for (i, &want) in expected.iter().enumerate() {
            assert_eq!(locs[i], want, "single-bit slot {i}");
        }
    }
}
