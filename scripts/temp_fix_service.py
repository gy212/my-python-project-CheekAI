from pathlib import Path

path = Path('backend/app/service.py')
text = path.read_text(encoding='latin-1')
old_split = '    parts = re.split(r"(?<=[\xe3\x80\x82\xef\xbc\x81\xef\xbc?!?])\\s+", text)'
new_split = '    parts = re.split(r"(?<=[\\u3002\\uFF01\\uFF1F?!])\\s+", text)'
if old_split not in text:
    raise SystemExit('split pattern not found')
text = text.replace(old_split, new_split, 1)
start = text.index('    if words:')
end = text.index('    puncts = re.findall', start)
replacement_block = "    if words:\n        func_words = {\n            \"\\u7684\",\n            \"\\u4e86\",\n            \"\\u548c\",\n            \"\\u4e00\",\n            \"\\u53c8\",\n            \"\\u5e76\",\n            \"\\u662f\",\n            \"\\u5728\",\n            \"\\u5c0d\",\n            \"\\u800c\",\n            \"\\u53ca\\u5176\",\n            \"\\u6216\\u8005\",\n            \"\\u4ee5\\u53ca\",\n            \"\\u5e76\\u4e14\",\n            \"\\u5982\\u679c\",\n            \"\\u56e0\\u70ba\",\n            \"\\u6240\\u4ee5\",\n            \"\\u4f46\\u662f\",\n        }\n        fw = sum(1 for w in words if w in func_words) / max(1, len(words))\n"
text = text[:start] + replacement_block + text[end:]
old_punct_line = '    puncts = re.findall(r"[\xef\xbc\x8c\xe3\x80\x82\xef\xbc\x81\xef\xbc?.!?]", text)'
if old_punct_line not in text:
    raise SystemExit('punct pattern not found')
text = text.replace(old_punct_line, '    puncts = re.findall(r"[\\uFF0C\\u3002\\uFF01\\uFF1F,.!?]", text)', 1)
path.write_text(text, encoding='latin-1')
