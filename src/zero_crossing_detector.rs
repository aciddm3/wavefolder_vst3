pub fn zero_crosing_points(table: &Vec<f32>, dest_vec: &mut Vec<f32>) {
	dest_vec.clear();
	
    if table.is_empty() {        
        return;
    }

    if table[table.len() - 1] * table[0] < 0.0 {
        dest_vec.push(0.0);
    }

    for index in 0..table.len() - 1 {
        if table[index] * table[index + 1] < 0.0 {
            let index_float_part =
                table[index].abs() / (table[index].abs() + table[index + 1].abs());
            dest_vec.push((index as f32 + index_float_part) / table.len() as f32);
        }
    }
}
