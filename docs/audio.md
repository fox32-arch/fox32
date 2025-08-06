# Audio controller

The audio controller manages 8 PCM audio channels and sums them together
to get the final audio output. The controller has a maximum output rate
of 48kHz and can handle 8-bit/16-bit PCM samples at any physical address in memory.

## Port mapping

Audio ports can be accessed from the range 0x80000600-0x800006FF.

All of the channel registers for the 8 channels are accessed at $00-$7F. The upper
nibble determines the channel number, while the lower number determines the register.

The range from $82-$FF is unused and reserved for future expansions.

Each channel has 3 volume controls: the channel volume, the left volume and the right volume.
The channel is scaled by the channel volume (0-127), then it is scaled according to the left
and right volumes (both 0-255).

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

### 0xX0: Sample position (Read)
Read this register to get the current position of the sample that is playing in the audio channel.

### 0xX1: Sample data (Read)
Read this register to get the current output sample of the audio channel (16-bit half).

### 0xX0, 0xX1: Sample start and end (Write)
Write a number to these registers to specify the start and end of the sample relative to
the controller's sample base.

### 0xX2, 0xX3: Loop point start and end (Write-only)
Write a number to these registers to specify the start and end of the sample relative to
the controller's sample base. Reading these registers will return a 0.

### 0xX4: Channel accumulator (Read)
Read this register to get the current value of the channel phase accumulator.

### 0xX4: Channel rate (Write)
Write a number here to specify the rate at which samples are fetched.
The theoretical range is from 0 to 4294967295 (2^32 - 1), however the recommended maximum is
65536, since higher rates may result in undefined behaviour.

A desired rate R can be calculated according to the formula: $\frac{R}{48000}\times2^{16}$.
To play a sample at a sample rate of 48kHz, calculate the rate: $\frac{48000}{48000}\times2^{16} = 65536$ 
and write the resulting rate of 65536 to this register.

### 0xX5: Audio channel control (Read, Write)

 bits   | description
--------|------------------
  31:10 | not used
  9     | 0=8-bit PCM, 1=16-bit PCM
  8     | enable
  7     | loop
  6:0   | volume (0-127)

Note that the audio controller, when using 16-bit PCM, fetches 16-bit data as a little-endian word.

### 0xX6: Audio panning control (Read, Write)

 bits   | description
--------|------------------
  15:8  | left volume (0-255)
  7:0   | right volume (0-255)

### 0x80: Audio controller sample base (Read, Write)
Read from this register to get the current base from which samples are fetched and write to it to set
the base.

### 0x81: Audio controller status (Read, Write)

 bits   | description
--------|------------------
  15:8  | buffer rate
  5:4   | buffer format
  1     | refill pending flag
  0     | buffer mode (0=use channels, 1=use buffer)

Bits 15 to 8 specify the rate at which samples from the buffer are fetched. The formula for calculating
the rate for a sample rate R is $\frac{R}{48000}\times2^{7}$. A value of 0 halts the internal counter and
no samples are fetched until the rate is non-zero.
  value    | description
-----------|--------------------
  128      | maximum rate (48kHz)
  64       | half rate (24kHz)
  32       | quarter rate (12khz)
  0        | stops playback
  \>128    | undefined behaviour

If bit 0 of this port is set, then the channels are disabled and the controller now expects
a contiguous area of 32 kilobytes (32768 bytes) for the audio buffer. This allows for
software-driven mixing and more control over the audio output.
The format of this buffer is determined by bits 4 and 5:
  value    | description
-----------|--------------------
  0b00 (0) | mono 8-bit
  0b01 (1) | mono 16-bit
  0b10 (2) | stereo 8-bit
  0b11 (3) | stereo 16-bit

Stereo samples are fetched as frames of left and right. For example, if the buffer is using stereo 8-bit PCM,
it fetches the left sample first (1 byte), then the right sample last (1 byte), advancing the internal position by 2 bytes.
For stereo 16-bit PCM, it fetches the left sample word first (2 bytes) and the right sample word last (2 bytes),
advancing the internal position by 4 bytes.

The audio controller will always use a 32768 byte buffer, regardless of the format.
The effective buffer size then is variable depending on the format:
  format        | size (bytes) | buffer size (samples)
----------------|--------------|---------------------------
  mono 8-bit    | 1            | 32768
  mono 16-bit   | 2            | 16384
  stereo 8-bit  | 2            | 16384
  stereo 16-bit | 4            | 8192

Whenever the internal position is half-way through the buffer, it raises an interrupt of vector 0xFE (address 0x000003F8).
When this interrupt is raised, the refill pending flag (bit 1) is set, and must be cleared
in order for future refill interrupts to happen. In that case, a value of 0 must be written
to clear the flag.