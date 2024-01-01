#!/Users/daruma/.pyenv/pyenv-win/versions/3.9.1/python.exe
import subprocess
res = subprocess.run(["cargo","test","--","--nocapture"]).returncode
if res != -1:
    input("\033[32mテスト終了\033[0m")
else: 
    input("\033[31mテスト終了\033[0m")
