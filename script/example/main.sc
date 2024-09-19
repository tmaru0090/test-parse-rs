@include "std.sc"

// main関数
fn main()->i32{
	l a = 1000*3;
        l b = a;
        l c = b;
        l d = c;
        return a;
}
// 関数実行
l ret = main();


