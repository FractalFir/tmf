# Tight Model Format Specification version 0.1
## 1. File structure
### 1.1 TMF Header
TMF Header consist of following data, laid out without padding.
1. Magic - 3 bytes ([0x54,0x4d,0x46]) creating the string "TMF" used for identification
2. TMF_MAJOR - major version of format used to save file, litl
3. TMF_MINOR - major version of format used to save file.
## 2 Data Types
All types in specification consist of 2 parts:
1. type prefix - denotes *what* data is stored
2. type size - denotes *size* of stored data in bits
Type prefix and type size are explained in depth in their respective sections
### 2.1 Enidianess
All data is little ennidian unless explicitly stated otherwise. 
### 2.2 Type Prefix
All types have a prefix identifing what data is stored inside that type. Used prefixes are:
|prefix|meaning|
|------|-------|
| f | IEEE 754 floating point type. |
| u | unsigned 

