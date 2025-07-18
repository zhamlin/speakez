/**
 * Represents a base constructor type for typed arrays.
 * This type defines the various constructor signatures that typed arrays can have.
 * 
 * @template T - The specific typed array type (e.g., Int8Array, Uint8Array).
 */
type TypedArrayConstructorBase<T> = {
  /**
   * Constructs a new typed array with no elements.
   * @returns A new typed array instance.
   */
  new (): T;

  /**
   * Constructs a new typed array with the specified length.
   * @param length - The length of the new typed array.
   * @returns A new typed array instance.
   */
  new (length: number): T;

  /**
   * Constructs a new typed array from an array-like or iterable object.
   * @param typedArray - An array-like or iterable object to initialize the typed array.
   * @returns A new typed array instance.
   */
  new (typedArray: ArrayLike<number> | Iterable<number>): T;

  /**
   * Constructs a new typed array from an object.
   * @param object - An object to initialize the typed array.
   * @returns A new typed array instance.
   */
  new (object: object): T;

  /**
   * Constructs a new typed array from an ArrayBuffer.
   * @param buffer - The ArrayBuffer to use as the storage for the typed array.
   * @returns A new typed array instance.
   */
  new (buffer: ArrayBufferLike): T;

  /**
   * Constructs a new typed array from an ArrayBuffer with a specified byte offset.
   * @param buffer - The ArrayBuffer to use as the storage for the typed array.
   * @param byteOffset - The offset, in bytes, to the first element in the array.
   * @returns A new typed array instance.
   */
  new (buffer: ArrayBufferLike, byteOffset: number): T;

  /**
   * Constructs a new typed array from an ArrayBuffer with a specified byte offset and length.
   * @param buffer - The ArrayBuffer to use as the storage for the typed array.
   * @param byteOffset - The offset, in bytes, to the first element in the array.
   * @param length - The number of elements in the array.
   * @returns A new typed array instance.
   */
  new (buffer: ArrayBufferLike, byteOffset: number, length: number): T;

  /**
   * The number of bytes per element in the typed array.
   */
  BYTES_PER_ELEMENT: number;
};

/**
 * Represents a constructor type for various typed arrays.
 * This type is a union of constructors for different typed arrays
 * such as Int8Array, Uint8Array, Uint8ClampedArray, Int16Array, Uint16Array,
 * Int32Array, Uint32Array, Float32Array, and Float64Array.
 */
type TypedArrayConstructor =
  | TypedArrayConstructorBase<Int8Array>
  | TypedArrayConstructorBase<Uint8Array>
  | TypedArrayConstructorBase<Uint8ClampedArray>
  | TypedArrayConstructorBase<Int16Array>
  | TypedArrayConstructorBase<Uint16Array>
  | TypedArrayConstructorBase<Int32Array>
  | TypedArrayConstructorBase<Uint32Array>
  | TypedArrayConstructorBase<Float32Array>
  | TypedArrayConstructorBase<Float64Array>;

/**
 * Represents a union type for various typed arrays.
 * This type includes all standard typed arrays such as:
 * - Int8Array: An array of 8-bit signed integers.
 * - Uint8Array: An array of 8-bit unsigned integers.
 * - Uint8ClampedArray: An array of 8-bit unsigned integers clamped to 0-255.
 * - Int16Array: An array of 16-bit signed integers.
 * - Uint16Array: An array of 16-bit unsigned integers.
 * - Int32Array: An array of 32-bit signed integers.
 * - Uint32Array: An array of 32-bit unsigned integers.
 * - Float32Array: An array of 32-bit floating point numbers.
 * - Float64Array: An array of 64-bit floating point numbers.
 */
type TypedArray = 
  | Int8Array
  | Uint8Array
  | Uint8ClampedArray
  | Int16Array
  | Uint16Array
  | Int32Array
  | Uint32Array
  | Float32Array
  | Float64Array;

  /**
   * Represents a callback function used in the writeCallback method of the RingBuffer class.
   * This callback is responsible for writing data to two separate storage buffers.
   * 
   * @param storageA - The first storage buffer as a typed array.
   * @param storageB - The second storage buffer as a typed array.
   * @returns The number of elements that have been written to the storage buffers.
   */
  type RingBufferWriteCallback = (storageA: TypedArray, storageB: TypedArray) => number;
  
  /**
   * Represents a callback function used in the writeCallbackWithOffset method of the RingBuffer class.
   * This callback is responsible for writing data to the ring buffer at specified offsets.
   * 
   * @param storage - The internal storage of the ring buffer as a typed array.
   * @param offsetStartWritingFrom - The offset to start writing from in the storage.
   * @param numElementsToWriteAtOffset - The number of elements to write at the first offset.
   * @param offsetStartWritingFromB - The second offset to start writing from in the storage.
   * @param numElementsToWriteAtOffsetB - The number of elements to write at the second offset.
   * @returns The number of elements that have been written to the storage.
   */
  type RingBufferWriteCallbackWithOffset = (
    storage: TypedArray, 
    offsetStartWritingFrom: number, 
    numElementsToWriteAtOffset: number, 
    offsetStartWritingFromB: number, 
    numElementsToWriteAtOffsetB: number
  ) => number;

/**
 * The base RingBuffer class
 *
 * A Single Producer - Single Consumer thread-safe wait-free ring buffer.
 *
 * The producer and the consumer can be on separate threads, but cannot change roles,
 * except with external synchronization.
 */
declare class RingBuffer {
    /** Allocate the SharedArrayBuffer for a RingBuffer, based on the type and
     * capacity required
     * @param capacity The number of elements the ring buffer will be
     * able to hold.
     * @param type A typed array constructor, the type that this ring
     * buffer will hold.
     * @return A SharedArrayBuffer of the right size.
     */
    static getStorageForCapacity(capacity: number, type: TypedArrayConstructor): SharedArrayBuffer;
    private _type;
    private _capacity;
    private buf;
    private write_ptr;
    private read_ptr;
    private storage;
    /**
     * @param sab A SharedArrayBuffer obtained by calling
     * {@link RingBuffer.getStorageForCapacity}.
     * @param type A typed array constructor, the type that this ring
     * buffer will hold.
     */
    constructor(sab: SharedArrayBuffer, type: TypedArrayConstructor);
    /**
     * @return the type of the underlying ArrayBuffer for this RingBuffer. This
     * allows implementing crude type checking.
     */
    type(): string;
    /**
     * Push elements to the ring buffer.
     * @param elements A typed array of the same type as passed in the ctor, to be written to the queue.
     * @param length If passed, the maximum number of elements to push.
     * If not passed, all elements in the input array are pushed.
     * @param offset If passed, a starting index in elements from which
     * the elements are read. If not passed, elements are read from index 0.
     * @return the number of elements written to the queue.
     */
    push(elements: TypedArray, length?: number, offset?: number): number;
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
    writeCallback(amount: number, cb: RingBufferWriteCallback): number;
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
    writeCallbackWithOffset(amount: number, cb: RingBufferWriteCallbackWithOffset): number;
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
    pop(elements: TypedArray, length?: number, offset?: number): number;
    /**
     * @return True if the ring buffer is empty false otherwise. This can be late
     * on the reader side: it can return true even if something has just been
     * pushed.
     */
    empty(): boolean;
    /**
     * @return True if the ring buffer is full, false otherwise. This can be late
     * on the write side: it can return true when something has just been popped.
     */
    full(): boolean;
    /**
     * @return The usable capacity for the ring buffer: the number of elements
     * that can be stored.
     */
    capacity(): number;
    /**
     * @return The number of elements available for reading. This can be late, and
     * report less elements that is actually in the queue, when something has just
     * been enqueued.
     */
    availableRead(): number;
    /**
     * Compatibility alias for availableRead().
     *
     * @return The number of elements available for reading. This can be late, and
     * report less elements that is actually in the queue, when something has just
     * been enqueued.
     *
     * @deprecated
     */
    available_read(): number;
    /**
     * @return The number of elements available for writing. This can be late, and
     * report less elements that is actually available for writing, when something
     * has just been dequeued.
     */
    availableWrite(): number;
    /**
     * Compatibility alias for availableWrite.
     *
     * @return The number of elements available for writing. This can be late, and
     * report less elements that is actually available for writing, when something
     * has just been dequeued.
     *
     * @deprecated
     */
    available_write(): number;
    /**
     * @return Number of elements available for reading, given a read and write
     * pointer.
     * @private
     */
    private _available_read;
    /**
     * @return Number of elements available from writing, given a read and write
     * pointer.
     * @private
     */
    private _available_write;
    /**
     * @return The size of the storage for elements not accounting the space for
     * the index, counting the empty slot.
     * @private
     */
    private _storage_capacity;
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
    private _copy;
}

/**
 * Interleaved -> Planar audio buffer conversion
 *
 * This is useful to get data from a codec, the network, or anything that is
 * interleaved, into a planar format, for example a Web Audio API AudioBuffer or
 * the output parameter of an AudioWorkletProcessor.
 *
 * @param input is an array of n*128 frames arrays, interleaved,
 * where n is the channel count.
 * @param output is an array of 128-frames arrays.
 */
declare function deinterleave(input: Float32Array, output: Float32Array[]): void;
/**
 * Planar -> Interleaved audio buffer conversion
 *
 * This function is useful to get data from the Web Audio API (that uses a
 * planar format), into something that a codec or network streaming library
 * would expect.
 *
 * @param input An array of n*128 frames Float32Array that hold the audio data.
 * @param output A Float32Array that is n*128 elements long.
 */
declare function interleave(input: Float32Array[], output: Float32Array): void;
/**
 * Send interleaved audio frames to another thread, wait-free.
 *
 * These classes allow communicating between a non-real time thread (browser
 * main thread or worker) and a real-time thread (in an AudioWorkletProcessor).
 * Write and Reader cannot change role after setup, unless externally
 * synchronized.
 *
 * GC _can_ happen during the initial construction of this object when hopefully
 * no audio is being output. This depends on how implementations schedule GC
 * passes. After the setup phase no GC is triggered on either side of the queue.
 */
declare class AudioWriter {
    private ringbuf;
    /**
     * From a RingBuffer, build an object that can enqueue enqueue audio in a ring
     * buffer.
     */
    constructor(ringbuf: RingBuffer);
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
    enqueue(buf: Float32Array): number;
    /**
     * @deprecated Use availableWrite() instead. This method is deprecated and will be removed in future versions.
     */
    available_write(): number;
    /**
     * @return The free space in the ring buffer. This is the amount of samples
     * that can be queued, with a guarantee of success.
     */
    availableWrite(): number;
}
/**
 * Receive interleaved audio frames to another thread, wait-free.
 *
 * GC _can_ happen during the initial construction of this object when hopefully
 * no audio is being output. This depends on how implementations schedule GC
 * passes. After the setup phase no GC is triggered on either side of the queue.
 */
declare class AudioReader {
    private ringbuf;
    /**
     * From a RingBuffer, build an object that can dequeue audio in a ring
     * buffer.
     */
    constructor(ringbuf: RingBuffer);
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
    dequeue(buf: Float32Array): number;
    /**
     * @deprecated Use availableRead() instead. This method is deprecated and will be removed in future versions.
     */
    available_read(): number;
    /**
     * Query the occupied space in the queue.
     *
     * @return The amount of samples that can be read with a guarantee of success.
     */
    availableRead(): number;
}

/**
 * Send parameter changes, lock free, no gc, between a UI thread (browser
 * main thread or worker) and a real-time thread (in an AudioWorkletProcessor).
 * Write and Reader cannot change roles after setup, unless externally
 * synchronized.
 *
 * GC _can_ happen during the initial construction of this object when hopefully
 * no audio is being output. This depends on the implementation.
 *
 * Parameter changes are like in the VST framework: an index and a float value
 * (no restriction on the value).
 *
 * This class supports up to 256 parameters, but this is easy to extend if
 * needed.
 *
 * An element is an index, that is an unsigned byte, and a float32, which is 4
 * bytes.
 */
declare class ParameterWriter {
    private ringbuf;
    private mem;
    private array;
    private view;
    /**
     * From a RingBuffer, build an object that can enqueue a parameter change in
     * the queue.
     * @param ringbuf A RingBuffer object of Uint8Array.
     */
    constructor(ringbuf: RingBuffer);
    /**
     * Enqueue a parameter change for parameter of index `index`, with a new value
     * of `value`.
     *
     * @param index The index of the parameter.
     * @param value The value of the parameter.
     * @return True if enqueuing succeeded, false otherwise.
     */
    enqueueChange(index: number, value: number): boolean;
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
    enqueue_change(index: number, value: number): boolean;
}
/**
 * Receive parameter changes, lock free, no gc, between a UI thread (browser
 * main thread or worker) and a real-time thread (in an AudioWorkletProcessor).
 * Write and Reader cannot change roles after setup, unless externally
 * synchronized.
 *
 * GC _can_ happen during the initial construction of this object when hopefully
 * no audio is being output. This depends on the implementation.
 *
 * Parameter changes are like in the VST framework: an index and a float value
 * (no restriction on the value).
 *
 * This class supports up to 256 parameters, but this is easy to extend if
 * needed.
 *
 * An element is an index, that is an unsigned byte, and a float32, which is 4
 * bytes.
 */
declare class ParameterReader {
    private ringbuf;
    private mem;
    private array;
    private view;
    /**
     * @param ringbuf A RingBuffer setup to hold Uint8.
     */
    constructor(ringbuf: RingBuffer);
    /**
     * Attempt to dequeue a single parameter change.
     * @param o An object with two attributes: `index` and `value`.
     * @return true if a parameter change has been dequeued, false otherwise.
     */
    dequeueChange(o: {
        index: number;
        value: number;
    }): boolean;
    /**
     * Attempt to dequeue a single parameter change.
     * @param o An object with two attributes: `index` and `value`.
     * @return true if a parameter change has been dequeued, false otherwise.
     *
     * @deprecated
     */
    dequeue_change(o: {
        index: number;
        value: number;
    }): boolean;
}

export { AudioReader, AudioWriter, ParameterReader, ParameterWriter, RingBuffer, deinterleave, interleave };
