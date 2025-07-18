export function haveSameProperties(obj1, obj2) {
	const obj1Keys = Object.keys(obj1);
	const obj2Keys = Object.keys(obj2);

	// Compare the length of the keys
	if (obj1Keys.length !== obj2Keys.length) {
		return false;
	}

	// Compare the keys themselves
	return obj1Keys.every((key) => key in obj2);
}

export function ifSameA(a, b) {
	if (haveSameProperties(a, b)) {
		return a;
	}
	return b;
}

/**
 * Recursively traverses an object and wraps all non-object fields with a function.
 *
 * @param {function(any): any} fn - The object to traverse.
 * @param {object} obj - The object to traverse.
 * @returns {object} A new object where all non-object fields are wrapped with signal().
 */
export function wrapNonObjectFieldsWithFn(fn, obj) {
	if (typeof obj !== "object" || obj === null) {
		return fn(obj); // Base case: if not an object, wrap in signal
	}

	const isArray = Array.isArray(obj);
	if (isArray) {
		return fn(obj);
	}

	const result = {};
	// const result = Array.isArray(obj) ? [] : {};
	for (const key in obj) {
		if (Object.hasOwn(obj, key)) {
			result[key] = wrapNonObjectFieldsWithFn(fn, obj[key]); // Recursively traverse
		}
	}
	return result;
}

export function isAllZero(array) {
	for (let i = 0; i < array.length; i++) {
		if (array[i] !== 0) {
			return false;
		}
	}
	return true;
}

export function clamp(number, min, max) {
	return Math.max(min, Math.min(number, max));
}

export function dynamicSort(property) {
	let sortOrder = 1;
	if (property[0] === "-") {
		sortOrder = -1;
		// biome-ignore lint:
		property = property.substr(1);
	}

	return (a, b) => {
		// works with string and numbers
		const result =
			a[property] < b[property] ? -1 : a[property] > b[property] ? 1 : 0;
		return result * sortOrder;
	};
}

export function timeoutPromise(ms) {
	return new Promise((_, reject) => {
		setTimeout(() => {
			reject(new Error("Operation timed out"));
		}, ms);
	});
}

/**
 * @template T
 * @param {Promise<T>} promise
 * @param {number} ms
 * @returns {Promise<T>}
 */
export function runWithTimeout(promise, ms) {
	return Promise.race([promise, timeoutPromise(ms)]);
}
