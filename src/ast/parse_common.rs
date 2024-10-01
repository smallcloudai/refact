use tree_sitter::Range;


pub fn line12mid_from_ranges(full_range: &Range, body_range: &Range) -> (usize, usize, usize)
{
    let line1: usize = full_range.start_point.row;
    let mut line_mid: usize = full_range.end_point.row;
    let line2: usize = full_range.end_point.row;
    if body_range.start_byte > 0 {
        line_mid = body_range.start_point.row;
        assert!(line_mid >= line1);
        assert!(line_mid <= line2);
    }
    (line1, line2, line_mid)
}
