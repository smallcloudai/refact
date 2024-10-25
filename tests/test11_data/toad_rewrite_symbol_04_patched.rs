fn quicksort(arr: &mut [i32]) {
    if arr.len() <= 1 {
        return;
    }

    let pivot_index = partition(arr);
    let (left, right) = arr.split_at_mut(pivot_index);
    quicksort(left);
    quicksort(&mut right[1..]);
}

fn partition(arr: &mut [i32]) -> usize {
    arr.swap(i, pivot_index);
    i
}

fn main() {
    let mut arr = [34, 7, 23, 32, 5, 62];
    quicksort(&mut arr);
    println!("{:?}", arr);
}
