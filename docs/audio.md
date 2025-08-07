# Audio controller

The audio controller manages 8 PCM audio channels, summing them together
to produce the final audio output. The controller has a maximum output rate
of 48 kHz and can handle 8-bit or 16-bit PCM samples from any physical address in memory.

Each channel has 3 volume controls: the channel volume, the left volume, and the right volume.
The channel is scaled by the channel volume (0-127), and then by the left and right volumes (both 0-255).

## Port mapping

Audio ports can be accessed within the range 0x80000600-0x800006FF.

Registers for the 8 audio channels are accessed at 0x00-0x7F. The upper nibble determines 
the channel number, while the lower nibble determines the register.

 offset (X = 0...7) | description
--------------------|------------------
  0xX0              | audio channel X sample start
  0xX1              | audio channel X sample end
  0xX2              | audio channel X loop point start
  0xX3              | audio channel X loop point end
  0xX4              | audio channel X rate
  0xX5              | audio channel X control
  0xX6              | audio channel X panning

Registers relating to the current status of the audio controller are located at ports 0x80 and 0x81.
 offset | description
--------|------------------
  0x80  | audio controller sample base
  0x81  | audio controller state

The range from 0x82-0xFF is unused and reserved for future expansions.

### 0xX0: Sample position (Read)
Read this register to get the current position of the sample being played in the audio channel.

### 0xX1: Sample data (Read)
Read this register to get the current 16-bit output sample of the audio channel.

### 0xX0, 0xX1: Sample start and end (Write)
Write a value to these registers to specify the start and end points of the sample relative to
the controller's sample base. The sample endpoint written here must be the endpoint + 1.
For example, if the sample is 8000 bytes long, then the start point should be 0 and the end point
should be 8000. This also applies to the loop points.

Note that these values are in bytes, and do not change depending on the format. If the start of
the sample is 0 and the end of the sample is 2, then the controller will always read 2 bytes, 
regardless of the bit-depth of the channel. This is valid for 8-bit PCM, but for a 16-bit sample
of the same length, the endpoint value must be 4.

### 0xX2, 0xX3: Loop point start and end (Write-only)
Write a value to these registers to specify the start and end of the sample relative to
the controller's sample base. Reading these registers will return a 0.

### 0xX4: Channel accumulator (Read)
Read this register to get the current value of the channel's phase accumulator.

### 0xX4: Channel rate (Write)
Write a value here to specify the rate at which samples are fetched.
Although any 32-bit value can be written here, giving a theoretical range of 0 to 4294967295 (2^32 - 1),
any values above 65536 will not change the frequency, and will max out at 48 kHz. The effective sampling
rate range is from 0 hz to 48 kHz.

A given frequency (F) can be calculated using the formula: $\frac{F}{48000}\times2^{16}$.
To play a sample at a sample rate of 48 kHz, calculate the rate as follows:
$\frac{48000}{48000}\times2^{16} = 65536$ 
and write the resulting rate of 65536 to this register.

### 0xX5: Audio channel control (Read, Write)

 bits   | description
--------|------------------
  31:10 | not used
  9     | 0=8-bit PCM, 1=16-bit PCM
  8     | enable
  7     | loop
  6:0   | volume (0-127)

When using 16-bit PCM, the audio controller fetches 16-bit data as a little-endian word.

### 0xX6: Audio panning control (Read, Write)

 bits   | description
--------|------------------
  15:8  | left volume (0-255)
  7:0   | right volume (0-255)

### 0x80: Audio controller sample base (Read, Write)
Read from this register to get the current base address from which samples are fetched and
write to it to set the base.

### 0x81: Audio controller status (Read, Write)

 bits   | description
--------|------------------
  15:8  | buffer rate
  5:4   | buffer format
  1     | refill pending flag (read-only)
  0     | buffer mode (0=use channels, 1=use buffer)

Bits 15 to 8 specify the rate at which samples are fetched from the buffer. The formula for calculating
the rate for a given sample rate (F) is $\frac{F}{48000}\times2^{7}$. A value of 0 halts the internal counter and
no samples are fetched until a non-zero rate is set.
  value    | description
-----------|--------------------
  128      | maximum rate (48 kHz)
  64       | half rate (24 kHz)
  32       | quarter rate (12 kHz)
  0        | stops playback
  \>128    | undefined behaviour

If bit 0 of this port is set, the channels are disabled and the controller expects
a contiguous area of 32 kilobytes (32,768 bytes) for the audio buffer. This allows for
software-driven mixing and provides more control over the audio output.
The format of this buffer is determined by bits 4 and 5:
  value    | description
-----------|--------------------
  0b00 (0) | mono 8-bit
  0b01 (1) | mono 16-bit
  0b10 (2) | stereo 8-bit
  0b11 (3) | stereo 16-bit

Stereo samples are fetched as frames containing left and right samples. For example, if the buffer is using stereo 8-bit PCM,
it fetches the left sample first (1 byte), then the right sample last (1 byte), advancing the internal position by 2 bytes.
For stereo 16-bit PCM, it fetches the left sample word first (2 bytes) and the right sample word last (2 bytes),
advancing the internal position by 4 bytes.

The audio controller always uses a 32768 byte buffer, regardless of the format.
Consequently, the effective buffer size is variable depending on the format:
  format        | size (bytes) | buffer size (samples)
----------------|--------------|---------------------------
  mono 8-bit    | 1            | 32768
  mono 16-bit   | 2            | 16384
  stereo 8-bit  | 2            | 16384
  stereo 16-bit | 4            | 8192

Whenever the internal position is half-way through the buffer, it raises an interrupt of vector 0xFE (address 0x000003F8).
When this interrupt is raised, the refill pending flag (bit 1) is set. The flag is cleared when the buffer wraps around to its beginning.
The processor cannot write to this flag; it can only read from it.