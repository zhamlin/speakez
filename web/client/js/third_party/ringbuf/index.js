function deinterleave(input, output) {
  const channel_count = input.length / 128;
  if (output.length !== channel_count) {
    throw new RangeError(
      `not enough space in output arrays ${output.length} != ${channel_count}`
    );
  }
  for (let i = 0; i < channel_count; i++) {
    const out_channel = output[i];
    let interleaved_idx = i;
    for (let j = 0; j < 128; j += 4) {
      out_channel[j] = input[interleaved_idx];
      out_channel[j + 1] = input[interleaved_idx + channel_count];
      out_channel[j + 2] = input[interleaved_idx + 2 * channel_count];
      out_channel[j + 3] = input[interleaved_idx + 3 * channel_count];
      interleaved_idx += 4 * channel_count;
    }
  }
}
function interleave(input, output) {
  if (input.length * 128 !== output.length) {
    throw new RangeError("input and output of incompatible sizes");
  }
  let out_idx = 0;
  for (let i = 0; i < 128; i++) {
    for (let channel = 0; channel < input.length; channel++) {
      output[out_idx] = input[channel][i];
      out_idx++;
    }
  }
}
class AudioWriter {
  /**
   * From a RingBuffer, build an object that can enqueue enqueue audio in a ring
   * buffer.
   */
  constructor(ringbuf) {
    if (ringbuf.type() !== "Float32Array") {
      throw new TypeError("This class requires a ring buffer of Float32Array");
    }
    this.ringbuf = ringbuf;
  }
  /**
   * Enqueue a buffer of interleaved audio into the ring buffer.
   *
   * Care should be taken to enqueue a number of samples that is a multiple of the
   * channel count of the audio stream.
   *
   * @param buf An array of interleaved audio frames.
   *
   * @return The number of samples that have been successfully written to the
   * queue. `buf` is not written to during this call, so the samples that
   * haven't been written to the queue are still available.
   */
  enqueue(buf) {
    return this.ringbuf.push(buf);
  }
  /**
   * @deprecated Use availableWrite() instead. This method is deprecated and will be removed in future versions.
   */
  available_write() {
    return this.availableWrite();
  }
  /**
   * @return The free space in the ring buffer. This is the amount of samples
   * that can be queued, with a guarantee of success.
   */
  availableWrite() {
    return this.ringbuf.availableWrite();
  }
}
class AudioReader {
  /**
   * From a RingBuffer, build an object that can dequeue audio in a ring
   * buffer.
   */
  constructor(ringbuf) {
    if (ringbuf.type() !== "Float32Array") {
      throw new TypeError("This class requires a ring buffer of Float32Array");
    }
    this.ringbuf = ringbuf;
  }
  /**
   * Attempt to dequeue at most `buf.length` samples from the queue. This
   * returns the number of samples dequeued. If greater than 0, the samples are
   * at the beginning of `buf`.
   *
   * Care should be taken to dequeue a number of samples that is a multiple of the
   * channel count of the audio stream.
   *
   * @param buf A buffer in which to copy the dequeued
   * interleaved audio frames.
   * @return The number of samples dequeued.
   */
  dequeue(buf) {
    if (this.ringbuf.empty()) {
      return 0;
    }
    return this.ringbuf.pop(buf);
  }
  /**
   * @deprecated Use availableRead() instead. This method is deprecated and will be removed in future versions.
   */
  available_read() {
    return this.availableRead();
  }
  /**
   * Query the occupied space in the queue.
   *
   * @return The amount of samples that can be read with a guarantee of success.
   */
  availableRead() {
    return this.ringbuf.availableRead();
  }
}

class ParameterWriter {
  /**
   * From a RingBuffer, build an object that can enqueue a parameter change in
   * the queue.
   * @param ringbuf A RingBuffer object of Uint8Array.
   */
  constructor(ringbuf) {
    if (ringbuf.type() !== "Uint8Array") {
      throw new TypeError("This class requires a ring buffer of Uint8Array");
    }
    const SIZE_ELEMENT = 5;
    this.ringbuf = ringbuf;
    this.mem = new ArrayBuffer(SIZE_ELEMENT);
    this.array = new Uint8Array(this.mem);
    this.view = new DataView(this.mem);
  }
  /**
   * Enqueue a parameter change for parameter of index `index`, with a new value
   * of `value`.
   *
   * @param index The index of the parameter.
   * @param value The value of the parameter.
   * @return True if enqueuing succeeded, false otherwise.
   */
  enqueueChange(index, value) {
    const SIZE_ELEMENT = 5;
    if (this.ringbuf.availableWrite() < SIZE_ELEMENT) {
      return false;
    }
    this.view.setUint8(0, index);
    this.view.setFloat32(1, value);
    return this.ringbuf.push(this.array) === SIZE_ELEMENT;
  }
  /**
   * Enqueue a parameter change for parameter of index `index`, with a new value
   * of `value`.
   *
   * @param index The index of the parameter.
   * @param value The value of the parameter.
   * @return True if enqueuing succeeded, false otherwise.
   *
   * @deprecated
   */
  enqueue_change(index, value) {
    return this.enqueueChange(index, value);
  }
}
class ParameterReader {
  /**
   * @param ringbuf A RingBuffer setup to hold Uint8.
   */
  constructor(ringbuf) {
    const SIZE_ELEMENT = 5;
    this.ringbuf = ringbuf;
    this.mem = new ArrayBuffer(SIZE_ELEMENT);
    this.array = new Uint8Array(this.mem);
    this.view = new DataView(this.mem);
  }
  /**
   * Attempt to dequeue a single parameter change.
   * @param o An object with two attributes: `index` and `value`.
   * @return true if a parameter change has been dequeued, false otherwise.
   */
  dequeueChange(o) {
    if (this.ringbuf.empty()) {
      return false;
    }
    const rv = this.ringbuf.pop(this.array);
    o.index = this.view.getUint8(0);
    o.value = this.view.getFloat32(1);
    return rv === this.array.length;
  }
  /**
   * Attempt to dequeue a single parameter change.
   * @param o An object with two attributes: `index` and `value`.
   * @return true if a parameter change has been dequeued, false otherwise.
   *
   * @deprecated
   */
  dequeue_change(o) {
    return this.dequeueChange(o);
  }
}

class RingBuffer {
  /** Allocate the SharedArrayBuffer for a RingBuffer, based on the type and
   * capacity required
   * @param capacity The number of elements the ring buffer will be
   * able to hold.
   * @param type A typed array constructor, the type that this ring
   * buffer will hold.
   * @return A SharedArrayBuffer of the right size.
   */
  static getStorageForCapacity(capacity, type) {
    if (!type.BYTES_PER_ELEMENT) {
      throw TypeError("Pass in an ArrayBuffer subclass");
    }
    const bytes = 8 + (capacity + 1) * type.BYTES_PER_ELEMENT;
    return new SharedArrayBuffer(bytes);
  }
  /**
   * @param sab A SharedArrayBuffer obtained by calling
   * {@link RingBuffer.getStorageForCapacity}.
   * @param type A typed array constructor, the type that this ring
   * buffer will hold.
   */
  constructor(sab, type) {
    if (type.BYTES_PER_ELEMENT === void 0) {
      throw TypeError("Pass a concrete typed array class as second argument");
    }
    this._type = type;
    this._capacity = (sab.byteLength - 8) / type.BYTES_PER_ELEMENT;
    this.buf = sab;
    this.write_ptr = new Uint32Array(this.buf, 0, 1);
    this.read_ptr = new Uint32Array(this.buf, 4, 1);
    this.storage = new type(this.buf, 8, this._capacity);
  }
  /**
   * @return the type of the underlying ArrayBuffer for this RingBuffer. This
   * allows implementing crude type checking.
   */
  type() {
    return this._type.name;
  }
  /**
   * Push elements to the ring buffer.
   * @param elements A typed array of the same type as passed in the ctor, to be written to the queue.
   * @param length If passed, the maximum number of elements to push.
   * If not passed, all elements in the input array are pushed.
   * @param offset If passed, a starting index in elements from which
   * the elements are read. If not passed, elements are read from index 0.
   * @return the number of elements written to the queue.
   */
  push(elements, length, offset = 0) {
    const rd = Atomics.load(this.read_ptr, 0);
    const wr = Atomics.load(this.write_ptr, 0);
    if ((wr + 1) % this._storage_capacity() === rd) {
      return 0;
    }
    const len = length !== void 0 ? length : elements.length;
    const to_write = Math.min(this._available_write(rd, wr), len);
    const first_part = Math.min(this._storage_capacity() - wr, to_write);
    const second_part = to_write - first_part;
    this._copy(elements, offset, this.storage, wr, first_part);
    this._copy(elements, offset + first_part, this.storage, 0, second_part);
    Atomics.store(
      this.write_ptr,
      0,
      (wr + to_write) % this._storage_capacity()
    );
    return to_write;
  }
  /**
   * Write bytes to the ring buffer using callbacks. This create wrapper
   * objects and can GC, so it's best to no use this variant from a real-time
   * thread such as an AudioWorklerProcessor `process` method.
   * The callback is passed two typed arrays of the same type, to be filled.
   * This allows skipping copies if the API that produces the data writes is
   * passed arrays to write to, such as `AudioData.copyTo`.
   * @param amount The maximum number of elements to write to the ring
   * buffer. If amount is more than the number of slots available for writing,
   * then the number of slots available for writing will be made available: no
   * overwriting of elements can happen.
   * @param cb A callback with two parameters, that are two typed
   * array of the correct type, in which the data need to be copied. If the
   * callback doesn't return anything, it is assumed all the elements
   * have been written to. Otherwise, it is assumed that the returned number is
   * the number of elements that have been written to, and those elements have
   * been written started at the beginning of the requested buffer space.
   *
   * @return The number of elements written to the queue.
   */
  writeCallback(amount, cb) {
    const rd = Atomics.load(this.read_ptr, 0);
    const wr = Atomics.load(this.write_ptr, 0);
    if ((wr + 1) % this._storage_capacity() === rd) {
      return 0;
    }
    const to_write = Math.min(this._available_write(rd, wr), amount);
    const first_part = Math.min(this._storage_capacity() - wr, to_write);
    const second_part = to_write - first_part;
    const first_part_buf = new this._type(
      this.storage.buffer,
      8 + wr * this.storage.BYTES_PER_ELEMENT,
      first_part
    );
    const second_part_buf = new this._type(
      this.storage.buffer,
      8 + 0,
      second_part
    );
    const written = cb(first_part_buf, second_part_buf) || to_write;
    Atomics.store(this.write_ptr, 0, (wr + written) % this._storage_capacity());
    return written;
  }
  /**
   * Write bytes to the ring buffer using a callback.
   *
   * This allows skipping copies if the API that produces the data writes is
   * passed arrays to write to, such as `AudioData.copyTo`.
   *
   * @param amount The maximum number of elements to write to the ring
   * buffer. If amount is more than the number of slots available for writing,
   * then the number of slots available for writing will be made available: no
   * overwriting of elements can happen.
   * @param cb A callback with five parameters:
   *
   * (1) The internal storage of the ring buffer as a typed array
   * (2) An offset to start writing from
   * (3) A number of elements to write at this offset
   * (4) Another offset to start writing from
   * (5) A number of elements to write at this second offset
   *
   * If the callback doesn't return anything, it is assumed all the elements
   * have been written to. Otherwise, it is assumed that the returned number is
   * the number of elements that have been written to, and those elements have
   * been written started at the beginning of the requested buffer space.
   * @return The number of elements written to the queue.
   */
  writeCallbackWithOffset(amount, cb) {
    const rd = Atomics.load(this.read_ptr, 0);
    const wr = Atomics.load(this.write_ptr, 0);
    if ((wr + 1) % this._storage_capacity() === rd) {
      return 0;
    }
    const to_write = Math.min(this._available_write(rd, wr), amount);
    const first_part = Math.min(this._storage_capacity() - wr, to_write);
    const second_part = to_write - first_part;
    const written = cb(this.storage, wr, first_part, 0, second_part) || to_write;
    Atomics.store(this.write_ptr, 0, (wr + written) % this._storage_capacity());
    return written;
  }
  /**
   * Read up to `elements.length` elements from the ring buffer. `elements` is a typed
   * array of the same type as passed in the ctor.
   * Returns the number of elements read from the queue, they are placed at the
   * beginning of the array passed as parameter.
   * @param elements An array in which the elements read from the
   * queue will be written, starting at the beginning of the array.
   * @param length If passed, the maximum number of elements to pop. If
   * not passed, up to elements.length are popped.
   * @param offset If passed, an index in elements in which the data is
   * written to. `elements.length - offset` must be greater or equal to
   * `length`.
   * @return The number of elements read from the queue.
   */
  pop(elements, length, offset = 0) {
    const rd = Atomics.load(this.read_ptr, 0);
    const wr = Atomics.load(this.write_ptr, 0);
    if (wr === rd) {
      return 0;
    }
    const len = length !== void 0 ? length : elements.length;
    const to_read = Math.min(this._available_read(rd, wr), len);
    const first_part = Math.min(this._storage_capacity() - rd, to_read);
    const second_part = to_read - first_part;
    this._copy(this.storage, rd, elements, offset, first_part);
    this._copy(this.storage, 0, elements, offset + first_part, second_part);
    Atomics.store(this.read_ptr, 0, (rd + to_read) % this._storage_capacity());
    return to_read;
  }
  /**
   * @return True if the ring buffer is empty false otherwise. This can be late
   * on the reader side: it can return true even if something has just been
   * pushed.
   */
  empty() {
    const rd = Atomics.load(this.read_ptr, 0);
    const wr = Atomics.load(this.write_ptr, 0);
    return wr === rd;
  }
  /**
   * @return True if the ring buffer is full, false otherwise. This can be late
   * on the write side: it can return true when something has just been popped.
   */
  full() {
    const rd = Atomics.load(this.read_ptr, 0);
    const wr = Atomics.load(this.write_ptr, 0);
    return (wr + 1) % this._storage_capacity() === rd;
  }
  /**
   * @return The usable capacity for the ring buffer: the number of elements
   * that can be stored.
   */
  capacity() {
    return this._capacity - 1;
  }
  /**
   * @return The number of elements available for reading. This can be late, and
   * report less elements that is actually in the queue, when something has just
   * been enqueued.
   */
  availableRead() {
    const rd = Atomics.load(this.read_ptr, 0);
    const wr = Atomics.load(this.write_ptr, 0);
    return this._available_read(rd, wr);
  }
  /**
   * Compatibility alias for availableRead().
   *
   * @return The number of elements available for reading. This can be late, and
   * report less elements that is actually in the queue, when something has just
   * been enqueued.
   *
   * @deprecated
   */
  available_read() {
    return this.availableRead();
  }
  /**
   * @return The number of elements available for writing. This can be late, and
   * report less elements that is actually available for writing, when something
   * has just been dequeued.
   */
  availableWrite() {
    const rd = Atomics.load(this.read_ptr, 0);
    const wr = Atomics.load(this.write_ptr, 0);
    return this._available_write(rd, wr);
  }
  /**
   * Compatibility alias for availableWrite.
   *
   * @return The number of elements available for writing. This can be late, and
   * report less elements that is actually available for writing, when something
   * has just been dequeued.
   *
   * @deprecated
   */
  available_write() {
    return this.availableWrite();
  }
  // private methods //
  /**
   * @return Number of elements available for reading, given a read and write
   * pointer.
   * @private
   */
  _available_read(rd, wr) {
    return (wr + this._storage_capacity() - rd) % this._storage_capacity();
  }
  /**
   * @return Number of elements available from writing, given a read and write
   * pointer.
   * @private
   */
  _available_write(rd, wr) {
    return this.capacity() - this._available_read(rd, wr);
  }
  /**
   * @return The size of the storage for elements not accounting the space for
   * the index, counting the empty slot.
   * @private
   */
  _storage_capacity() {
    return this._capacity;
  }
  /**
   * Copy `size` elements from `input`, starting at offset `offset_input`, to
   * `output`, starting at offset `offset_output`.
   * @param input The array to copy from
   * @param offset_input The index at which to start the copy
   * @param output The array to copy to
   * @param offset_output The index at which to start copying the elements to
   * @param size The number of elements to copy
   * @private
   */
  _copy(input, offset_input, output, offset_output, size) {
    if (!size) {
      return;
    }
    if (offset_input === 0 && offset_output + input.length <= this._storage_capacity() && input.length === size) {
      output.set(input, offset_output);
      return;
    }
    let i = 0;
    const unrollFactor = 16;
    for (; i <= size - unrollFactor; i += unrollFactor) {
      output[offset_output + i] = input[offset_input + i];
      output[offset_output + i + 1] = input[offset_input + i + 1];
      output[offset_output + i + 2] = input[offset_input + i + 2];
      output[offset_output + i + 3] = input[offset_input + i + 3];
      output[offset_output + i + 4] = input[offset_input + i + 4];
      output[offset_output + i + 5] = input[offset_input + i + 5];
      output[offset_output + i + 6] = input[offset_input + i + 6];
      output[offset_output + i + 7] = input[offset_input + i + 7];
      output[offset_output + i + 8] = input[offset_input + i + 8];
      output[offset_output + i + 9] = input[offset_input + i + 9];
      output[offset_output + i + 10] = input[offset_input + i + 10];
      output[offset_output + i + 11] = input[offset_input + i + 11];
      output[offset_output + i + 12] = input[offset_input + i + 12];
      output[offset_output + i + 13] = input[offset_input + i + 13];
      output[offset_output + i + 14] = input[offset_input + i + 14];
      output[offset_output + i + 15] = input[offset_input + i + 15];
    }
    for (; i < size; i++) {
      output[offset_output + i] = input[offset_input + i];
    }
  }
}

export { AudioReader, AudioWriter, ParameterReader, ParameterWriter, RingBuffer, deinterleave, interleave };
