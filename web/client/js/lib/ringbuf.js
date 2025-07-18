/** @typedef {RingBuffer} RingBuffer */

export {
	RingBuffer,
	AudioReader,
	interleave,
	deinterleave,
	AudioWriter,
	ParameterReader,
	ParameterWriter,
} from "/js/third_party/ringbuf/index.js";

/**
 * A class to handle resumable reading from the ring buffer.
 */
export class ResumableReader {
	/**
	 * @param {RingBuffer} reader
	 * @param {Float32Array} buffer
	 */
	constructor(reader, buffer, tag) {
		this.reader = reader;
		this.buffer = buffer;

		this.size = null; // The expected size of the message
		this.elementsRead = 0; // How many elements have been read
		this.tag = tag;
	}

	/**
	 * Reads a sized message from the ring buffer. If the full message is not available, it can resume later.
	 *
	 * @returns {Float32Array|null} - The full message when available, or null if more data is needed.
	 */
	readSizedMessage() {
		// If size hasn't been read, attempt to read it
		if (this.size === null) {
			if (this.reader.availableRead() < 1) {
				return null; // Not enough data to read the size
			}

			// Read the size (1 element)
			this.reader.pop(this.buffer, 1);
			this.size = this.buffer[0];

			if (this.size > this.buffer.length) {
				throw `mumble: mic message is larger than staging buffer, got: ${size}`;
			}
		}

		// Now, attempt to read the message data
		const remainingElements = this.size - this.elementsRead;
		const availableToRead = Math.min(
			remainingElements,
			this.reader.availableRead(),
		);

		if (availableToRead > 0) {
			// Read the available portion of the message
			this.reader.pop(this.buffer, availableToRead, this.elementsRead);
			this.elementsRead += availableToRead;
		}

		// If the entire message has been read, return it
		if (this.elementsRead === this.size) {
			const buf = this.buffer.subarray(0, this.size);
			this.resetState();
			return buf;
		}

		// Otherwise, return null and wait for more data
		return null;
	}

	/**
	 * Resets the state for reading the next message.
	 */
	resetState() {
		this.size = null;
		this.elementsRead = 0;
	}
}

/**
 * Writes a sized message to the ring buffer.
 * The first element is the size of the message (number of Float32 values).
 *
 * @param {Float32Array} message - The message to send.
 * @param {RingBuffer} writer - The writer instance for the ring buffer.
 * @returns {boolean} - Whether the message was successfully written.
 */
export function writeSizedMessage(message, writer) {
	const size = message.length;
	if (size === undefined) {
		throw "invalid message for writeSizedMessage";
	}

	// Ensure there is enough space in the ring buffer (1 for size + message length)
	if (writer.availableWrite() < size + 1) {
		return false; // Not enough space in the ring buffer.
	}

	// First, write the size (1 element)
	writer.push([size]);

	// Then, write the actual message data
	writer.push(message);

	return true;
}
