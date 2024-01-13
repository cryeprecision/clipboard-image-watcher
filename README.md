# clipboard-image-watcher

Detect images in the clipboard and save them as `.png` files.

## How It Works

Register a message-only window using `CreateWindowExA` and listen for clipboard changes in the
`WindowProc` callback. If the new clipboard data is a bitmap, encode it as a PNG and write it to disk.
