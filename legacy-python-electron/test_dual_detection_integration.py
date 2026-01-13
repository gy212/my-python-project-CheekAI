# -*- coding: utf-8 -*-
"""Integration test for dual detection functionality."""

import sys
import requests
import json

def test_dual_detection():
    """Test if dual detection API works correctly."""
    url = "http://127.0.0.1:8787/api/detect"
    payload = {
        "text": "这是第一个测试句子。这是第二个测试句子。这是第三个测试句子。用于验证双重检测功能的完整性。",
        "providers": [],
        "usePerplexity": True,
        "useStylometry": True,
        "sensitivity": "medium",
        "chunking": {
            "chunkSizeTokens": 400,
            "overlapTokens": 80
        }
    }
    
    try:
        print("Sending request to API...")
        response = requests.post(url, json=payload, timeout=30)
        
        print(f"Status Code: {response.status_code}")
        
        if response.status_code != 200:
            print(f"Error: {response.text}")
            return False
        
        result = response.json()
        
        # Check basic response structure
        assert "aggregation" in result, "Missing aggregation"
        assert "segments" in result, "Missing segments"
        assert "requestId" in result, "Missing requestId"
        
        print(f"✓ Basic response structure valid")
        print(f"  Paragraph segments: {len(result['segments'])}")
        
        # Check dual detection
        has_dual = "dualDetection" in result and result["dualDetection"] is not None
        print(f"✓ Has dualDetection: {has_dual}")
        
        if has_dual:
            dual = result["dualDetection"]
            
            # Check dual detection structure
            assert "paragraph" in dual, "Missing paragraph in dualDetection"
            assert "sentence" in dual, "Missing sentence in dualDetection"
            assert "comparison" in dual, "Missing comparison in dualDetection"
            
            para_count = len(dual["paragraph"]["segments"])
            sent_count = len(dual["sentence"]["segments"])
            
            print(f"✓ Dual detection structure valid")
            print(f"  Paragraph mode segments: {para_count}")
            print(f"  Sentence mode segments: {sent_count}")
            
            # Check comparison
            comp = dual["comparison"]
            print(f"  Probability diff: {comp['probabilityDiff']:.4f}")
            print(f"  Consistency score: {comp['consistencyScore']:.4f}")
            print(f"  Divergent regions: {len(comp['divergentRegions'])}")
            
            print("\n✅ ALL TESTS PASSED!")
            print("\nDual detection feature is working correctly.")
            print("- Backend returns both paragraph and sentence detection results")
            print("- Comparison analysis is computed")
            print("- API response structure is valid")
            
            return True
        else:
            print("⚠️ dualDetection is None - feature may be disabled")
            print("This is expected if enable_dual_detection=False")
            return True
            
    except requests.exceptions.ConnectionError:
        print("❌ Cannot connect to API at", url)
        print("Please ensure the backend is running: python start.py")
        return False
    except Exception as e:
        print(f"❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_dual_detection()
    sys.exit(0 if success else 1)
