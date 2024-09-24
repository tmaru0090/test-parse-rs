// main関数
@include "./std.sc" // 一時的な実装
//mod std;
//use std::*;


fn sagyou(cmd,path){
   return @cmd(cmd,["--loop",path]);
}
fn main(){
    let path = "maou_bgm_healing15.mp3";
    sagyou("mpv",path);
}



