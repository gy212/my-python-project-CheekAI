from pathlib import Path
text = Path('backend/app/service.py').read_text(encoding='latin-1')
line = [ln for ln in text.splitlines() if 'parts = re.split' in ln][0]
print('LINE:', line.encode('unicode_escape'))
old = '    parts = re.split(r"(?<=[\\xe3\\x80\\x82\\xef\\xbc\\x81\\xef\\xbc?!?])\\\\s+", text)'
print('OLD :', old.encode('unicode_escape'))
print('EQUAL?', line == old)
