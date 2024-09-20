
@include "std.sc"

// main関数
fn main()->i32{
	l a = 100;
        l mut b = &a;
        b = 1919;
        return a;
}
// 関数実行
main();

