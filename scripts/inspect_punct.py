from pathlib import Path
text = Path('backend/app/service.py').read_text(encoding='latin-1')
line = [ln for ln in text.splitlines() if 'puncts = re.findall' in ln][0]
print(line.encode('unicode_escape'))
