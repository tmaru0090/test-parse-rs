@include "std.sc"
@include "example/test.sc"
fn music_play_lists(paths,sleep_time){
    for path in paths{
        @play_music(path);
        @sleep(sleep_time);
    }
}
fn main(){
    let paths = [
                "C:/Users/tanukimaru/Downloads/sounds/dewprism-forest.wav",
                "C:/Users/tanukimaru/Downloads/sounds/dewprism-town.wav",
                "C:/Users/tanukimaru/Downloads/sounds/kisida-keizai.wav",
                "C:/Users/tanukimaru/Downloads/sounds/maou_bgm_healing15.mp3",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw-densetu.wav",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw-end.mp3",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw-end.wav",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw-sentou-zimu.wav",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw-syouri-mokuzen.wav",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw-t-sentou.wav",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw-title.wav",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw-y-sentou.wav",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw2-op.wav",
                "C:/Users/tanukimaru/Downloads/sounds/pokemon-bw2-title.wav",
                "C:/Users/tanukimaru/Downloads/sounds/ryokugan.wav",
                "C:/Users/tanukimaru/Downloads/sounds/th3_05.mp3",
                "C:/Users/tanukimaru/Downloads/sounds/write-san-end (1).wav",
                "C:/Users/tanukimaru/Downloads/sounds/write-san-end (2).wav",
                "C:/Users/tanukimaru/Downloads/sounds/write-san-end.wav",
                "C:/Users/tanukimaru/Downloads/sounds/繧ｫ繝ｼ繧ｽ繝ｫ遘ｻ蜍・.mp3"];
   
    music_play_lists(paths,4);
    
}
