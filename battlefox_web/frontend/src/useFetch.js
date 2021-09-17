import { useEffect, useState } from "react";

const useFetch = (url) => {
	const [data, setData] = useState(null);
	const [isPending, setIsPending] = useState(true);
	const [error, setError] = useState(null);

	useEffect(() => {
		const abortCont = new AbortController();

		fetch(url, { signal: abortCont.signal })
			.then(resp => {
				if (!resp.ok) {
					throw Error('could not fetch the data for that resourse, sry');
				}
				return resp.json();
			})
			.then(data => {
				setIsPending(false);
				setData(data);
				setError(null);
			})
			.catch(e => {
				if (e.name === 'AbortError') {
					console.log('fetch aborted')
				} else {
					setError(e.message);
					setIsPending(false);
				}
			});

		return () => abortCont.abort();
	}, [url]);

	return { data, isPending, error };
};

export default useFetch;