# I/O bus

The I/O bus is accessed using the `in` and `out` instructions.
Every access is 32 bits wide; there is no byte-addressing on the I/O bus and
addresses don't have to be aligned to multiples of four.

When a peripheral has multiple instances (e.g. 32 display overlays, 4 disks),
the lowest bits of the address usually indicate the instance number, while
higher bits indicate the function to be performed.

|  start     |  end       | description
|------------|------------|---------------------------------------
| 0x00000000 | 0x00000000 | serial port
| 0x80000000 | 0x8000031f | display
| 0x80000400 | 0x80000401 | mouse
| 0x80000500 | 0x80000500 | keyboard
| 0x80000600 | 0x80000600 | audio
| 0x80000700 | 0x80000706 | RTC and uptime
| 0x80001000 | 0x80005003 | disk
| 0x80010000 | 0x80010000 | power controller


## 0x00000000: Serial port

### 0x00: Data register

Writing to this register outputs the lowest 8 bits as a byte on the debug
serial port (stdout in case of the fox32 emulator). If necessary, the CPU is
stalled long enough to ensure that the byte is output rather than discarded.

Reading from this register gets a byte of input from the serial port, if
available, or zero otherwise.


## 0x80000000: Display

The display controller manages a background framebuffer and 32 overlays,
and composes them into the picture seen on screen.

The background framebuffer is located at the fixed RAM address 0x2000000, or 32 MiB.
The screen resolution is 640x480 (307200 pixels, 1228800 bytes).

The pixel format is RGBA. The alpha channel determines transparency in overlays
insofar that a non-zero alpha value causes a pixel to be fully opaque and a
zero alpha value causes it to be fully transparent.

When multiple overlays are active at a specific pixel position, the highest
numbered overlay that provides an opaque pixel "wins": Its pixel RGB value is
output on screen.

### 0x00-0x1f: Overlay position

 bits   | description
--------|------------------
 31:16  | Y (vertical) position
 15:0   | X (horizontal) position

### 0x100-0x11f: Overlay size

 bits   | description
--------|------------------
 31:16  | height (vertical size)
 15:0   | width (horizontal size)

### 0x200-0x21f: Overlay buffer pointer

RAM address of the framebuffer for this overlay.

### 0x300-0x31f: Overlay enable

- Write non-zero to enable this overlay, zero to disable
- Read 1 if enabled, 0 of disabled


## 0x80000400: Mouse

The mouse position and button states can be read and written, but hardware may
change them on incoming mouse events.

### 0x00: Button states

 bits   | description
--------|------------------
  2     | mouse button is currently held
  1     | mouse button has been released
  0     | mouse button has been pressed

### 0x01: Mouse position

 bits   | description
--------|------------------
 31:16  | Y (vertical) position
 15:0   | X (horizontal) position


## 0x80000500: Keyboard

### 0x00: Get key event (read-only)

Read this register to get a key event from the key event queue, or 0 if none is
currently available.

 bits   | description
--------|------------------
  8     | 0=pressed, 1=released
  7:0   | PC-compatible keyboard scancode


## 0x80000600: Audio

TODO


## 0x80000700: Real-Time Clock (RTC) and Uptime

 offset | description
--------|------------------
  0x00  | year (e.g. 2023)
  0x01  | month (1-12)
  0x02  | day (1-31)
  0x04  | minute (0-59)
  0x05  | second (0-59)
  0x06  | milliseconds since startup


## 0x80001000: Disk

The disk controller supports four disk ports. Each of them may be connected to
a disk (or not) at any time.

Disks are organized in sectors of 512 bytes each.

### 0x80001000-0x80001003: Disk size (read-only)

Read this register to get the disk size in bytes.

### 0x80002000-0x80002003: Buffer pointer

Address of RAM buffer to be used for read/write transfers.

### 0x80003000-0x80003003: Initiate disk read (write-only)

Write a sector number to this register to initate a disk read from the
specified sector. The CPU is stalled until the read completes.

### 0x80004000-0x80004003: Initiate disk write (write-only)

Write a sector number to this register to initate a disk write to the specfied
sector. The CPU is stalled until the write completes.

### 0x80005000-0x80005003: Remove disk (write-only)

Write any value to this register to remove or eject the specified disk.


## 0x80010000: Power controller

### 0x00: Poweroff

Write 0 to this register to power the computer off (or terminate the emulator).

Poweroff does not take effect immediately, so software should do something safe
(such as spinning in a loop) after triggering the poweroff.
