-- いりす症候群のシミュレーション

title_max_n = 2;
album_max_n = 4;
-- BGMを変更 
select("Title",3,0,0.1)
press("LClick",0,0.1)

-- 最初にあるばむに選択肢を合わせる
select("Title",1,0,0.1)
press("LClick",0,0.1)

-- あるばむをすべて見る
for album_i = 0,album_max_n do
	select("Album",album_i,0,0.1)
	press("LClick",0,5)
	press("RClick",0,0.1)
end

