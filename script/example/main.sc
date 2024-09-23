// main関数
@include "./std.sc"
@include "example/test.sc"


fn test(){
    let _path = "C:/Users/tanukimaru/Downloads/bgm/"+"pokemon-bw-sentou-zimu.wav";
    l _out = @cmd("mpv",[_path]);
    l ret =&[100,100];
    l path = "C:\\Users\\tanukimaru\\Downloads\\fluidsynth-2.3.5-win10-x64\\bin\\fluidsynth";
    l args = ["-i","C:\\ProgramData\\soundfonts\\THFont.sf2","C:\\Users\\tanukimaru\\Downloads\\th1202.mid"];
    l out = @cmd(path,args);
}
fn main(){
    test();
}

