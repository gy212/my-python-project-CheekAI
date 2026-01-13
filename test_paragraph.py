# -*- coding: utf-8 -*-
"""Test wtpsplit paragraph segmentation"""
import requests
import json

text = """人工智能正在改变我们的生活方式。从智能手机到自动驾驶汽车，AI的应用无处不在。机器学习算法让推荐系统更加精准，自然语言处理让人机交互更加自然。

然而，人工智能也带来了许多挑战和风险。隐私泄露、算法偏见、就业冲击都是我们需要面对的问题。如何在发展AI的同时保护人类的权益，成为了一个重要的议题。

除了技术层面，我们还需要思考AI的伦理问题。机器人应该有权利吗？AI做出的决策应该由谁负责？这些问题都需要社会各界共同讨论。"""

# 测试分段 API
resp = requests.post('http://127.0.0.1:8788/paragraph', json={
    'text': text, 
    'language': 'zh', 
    'threshold': 0.3
})
print(f'Status: {resp.status_code}')
if resp.status_code == 200:
    data = resp.json()
    paragraphs = data['paragraphs']
    print(f'Got {len(paragraphs)} paragraphs:')
    for i, p in enumerate(paragraphs):
        preview = p['text'][:60] + '...' if len(p['text']) > 60 else p['text']
        print(f'  [{i}] ({p["start"]}-{p["end"]}) {preview}')
else:
    print(f'Error: {resp.text}')

# 测试分段块 API
print('\n--- Testing paragraph blocks ---')
resp2 = requests.post('http://127.0.0.1:8788/paragraph/blocks', json={
    'text': text, 
    'language': 'zh', 
    'threshold': 0.3,
    'minChars': 100,
    'targetChars': 300,
    'maxChars': 500
})
print(f'Status: {resp2.status_code}')
if resp2.status_code == 200:
    data2 = resp2.json()
    blocks = data2['blocks']
    print(f'Got {len(blocks)} blocks:')
    for b in blocks:
        preview = b['text'][:60] + '...' if len(b['text']) > 60 else b['text']
        print(f'  [{b["index"]}] paragraphs={b["paragraphCount"]} ({b["start"]}-{b["end"]}) {preview}')
else:
    print(f'Error: {resp2.text}')
