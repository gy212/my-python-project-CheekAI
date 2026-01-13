// File Handler Composable
// Handles file selection and text extraction

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export function useFileHandler() {
  // State
  const fileName = ref("未选择任何文件");
  const fileInput = ref<HTMLInputElement | null>(null);

  // Methods
  function triggerFileSelect() {
    fileInput.value?.click();
  }

  async function handleFileSelect(event: Event, onTextLoaded: (text: string) => void) {
    const target = event.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
      const file = target.files[0];
      fileName.value = file.name;
      const lowerName = file.name.toLowerCase();
      
      // Only read .txt files as text; DOCX needs backend processing
      if (lowerName.endsWith('.txt')) {
        const reader = new FileReader();
        reader.onload = (e) => {
          if (e.target?.result) {
            onTextLoaded(e.target.result as string);
          }
        };
        reader.onerror = () => {
          alert('文件读取失败，请重试');
        };
        reader.readAsText(file, 'UTF-8');
      } else if (lowerName.endsWith('.docx') || lowerName.endsWith('.pdf')) {
        // For DOCX and PDF, call backend to extract text
        try {
          onTextLoaded("正在解析文档...");
          const arrayBuffer = await file.arrayBuffer();
          const fileData = Array.from(new Uint8Array(arrayBuffer));
          const extractedText = await invoke("preprocess_file", {
            fileName: file.name,
            fileData: fileData,
          });
          onTextLoaded(extractedText as string);
        } catch (err: any) {
          console.error("Document parsing error:", err);
          onTextLoaded(`[解析文件失败: ${file.name}]

错误: ${err.message || err}

请直接粘贴文本内容。`);
        }
      } else {
        // For other files, try to read as text
        const reader = new FileReader();
        reader.onload = (e) => {
          if (e.target?.result) {
            onTextLoaded(e.target.result as string);
          }
        };
        reader.onerror = () => {
          onTextLoaded(`[无法读取文件: ${file.name}]`);
        };
        reader.readAsText(file, 'UTF-8');
      }
    } else {
      fileName.value = "未选择任何文件";
    }
  }

  function resetFile() {
    fileName.value = "未选择任何文件";
    if (fileInput.value) {
      fileInput.value.value = "";
    }
  }

  return {
    // State
    fileName,
    fileInput,
    // Methods
    triggerFileSelect,
    handleFileSelect,
    resetFile,
  };
}
