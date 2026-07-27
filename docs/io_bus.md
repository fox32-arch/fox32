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
| 0x80000600 | 0x800006ff | audio
| 0x80000700 | 0x8000070f | RTC and uptime, interval timers
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
  7     | 0=pressed, 1=released
  6:0   | PC-compatible keyboard scancode


## 0x80000600-0x80000681: Audio

All of the channel registers for the 8 channels are accessed at $00-$7F. The upper
nibble determines the channel number, while the lower number determines the register.

The range from $82-$FF is unused and reserved for future expansions.

 offset (X = 0...7) | description
--------------------|------------------
  0xX0              | audio channel X sample start
  0xX1              | audio channel X sample end
  0xX2              | audio channel X loop point start
  0xX3              | audio channel X loop point end
  0xX4              | audio channel X rate
  0xX5              | audio channel X control
  0xX6              | audio channel X panning
  0x80              | audio controller sample base
  0x81              | audio controller state
  0x82              | audio controller buffer 0 start address
  0x83              | audio controller buffer 1 start address
  0x84              | audio controller buffer length

For more details, see [audio.md](audio.md).

## 0x80000700: Real-Time Clock (RTC) and Uptime, Interval Timers

 offset | description
--------|------------------
  0x00  | year (e.g. 2023)
  0x01  | month (1-12)
  0x02  | day (1-31)
  0x03  | hour (0-23)
  0x04  | minute (0-59)
  0x05  | second (0-59)
  0x06  | milliseconds since startup
  0x07  | daylight savings time active (zero or non-zero)
  0x08  | instructions since startup
  0x09  | tick counter
  0x0A  | tick base 
  0x0B  | timer control
  0x0C  | timer A period (W), counter (R)
  0x0D  | timer B period (W), counter (R)
  0x0E  | timer C period (W), counter (R)
  0x0F  | timer D period (W), counter (R)

### 0x09: Tick counter (read-only)

This register increments by 1 every N instructions, where N is the value in the tick base register.

### 0x0A: Tick base

This register sets the interval at which tick counter is increased.

### 0x0B: Timer control

All bits are grouped together in the order: Timer D -> Timer C -> Timer B -> Timer A.

 bits   | description
--------|------------------
  23:20 | 0=disable interrupt, 1=enable interrupt
  19:16 | (write-only) 1=force reload the corresponding timer
  15:12 | 0=one-shot, 1=continuous
  11:8  | 0=clock per tick, 1=clock per instructions
  7:4   | 0=no interrupt pending, 1=interrupt pending
  3:0   | 0=disable timer, 1=enable timer

Timers count down from the period, set in ports 0x0C-0x0F, to 0. An interrupt is triggered during the
transition from 1 to 0. Writing to the timer's period port will not reset the counter, and the new
period will only take effect after its current period has finished. The counter can be reset with 
the new period by writing a 1 to bits 16-19 to force reload it.
During a timer interrupt, the handler must acknowledge the interrupt by writing a 0 to bits 4-7.
This can be done with the following:
```avrasm
in r0, 0x8000070B
bcl r0, 4 ; acknowledge timer A interrupt
out 0x8000070B, r0
```
It should be noted that the timers are only useful for scheduling code to be executed at certain
intervals, because they are ticked according to CPU instruction ticks, rather than real time.
Because instructions are emulated in short bursts, they are not guaranteed to be temporally accurate
at sub-frame intervals. As such, it is not recommended to use them for precise timekeeping applications.

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
