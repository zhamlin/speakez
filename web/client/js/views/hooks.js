import { useEffect, useRef } from "preact/hooks";

const useOnInit = (callback) => useEffect(callback, []);

// Taken from: https://css-tricks.com/using-requestanimationframe-with-react-hooks/
export const useAnimationFrame = (callback) => {
	// Use useRef for mutable variables that we want to persist
	// without triggering a re-render on their change
	const requestRef = useRef();
	const previousTimeRef = useRef();

	const animate = (time) => {
		if (previousTimeRef.current !== undefined) {
			const deltaTime = time - previousTimeRef.current;
			callback(deltaTime);
		}
		previousTimeRef.current = time;
		requestRef.current = requestAnimationFrame(animate);
	};

	useOnInit(() => {
		requestRef.current = requestAnimationFrame(animate);
		return () => cancelAnimationFrame(requestRef.current);
	});
};

export const useTrackRenderCount = (name) => {
	const renderCount = useRef(0);

	// Increment render count every time the component renders
	renderCount.current += 1;

	useEffect(() => {
		console.log(
			`Component '${name}' has rendered ${renderCount.current} times`,
		);
	});
};

export const useEventListener = (event, callback) => {
	useEffect(() => {
		window.addEventListener(event, callback);
		return () => window.removeEventListener(event, callback);
	}, [event, callback]);
};

export const useInterval = (callback, delay) => {
	useEffect(() => {
		const id = setInterval(callback, delay);
		return () => clearInterval(id);
	}, [callback, delay]);
};
