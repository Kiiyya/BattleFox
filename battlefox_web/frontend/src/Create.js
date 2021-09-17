import { useState } from "react";
import { useHistory } from "react-router";

const Create = () => {
	const [title, setTitle] = useState('');
	const [body, setBody] = useState('');
	const [author, setAuthor] = useState('mario');
	const [isPending, setIsPending] = useState(false);
	const history = useHistory();

	const handleSubmit = e => {
		e.preventDefault();
		const blog = { title, body, author };
		fetch('http://localhost:8000/blogs', {
			method: 'POST',
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(blog),
		})
		.then(x => x.json())
		.then(blog => {
			setIsPending(false);
			// history.go(-1);
			history.push('/blogs/' + blog.id);
		});
	}

	return (
		<div className="create">
			<h2>Create a new blog post</h2>
			<form onSubmit={handleSubmit}>
				<label>Blog Title:</label>
				<input
					type="text"
					required
					value={title}
					onChange={ev => setTitle(ev.target.value)}
				/>
				<label>Blog body:</label>
				<textarea
					required
					value={body}
					onChange={e => setBody(e.target.value)}
				></textarea>
				<label>Blog author:</label>
				<select value={author} onChange={ev => setAuthor(ev.target.value)}>
					<option value="mario">mario</option>
					<option value="yoshi">yoshi</option>
				</select>
				{!isPending && <button>Add Blog</button>}
				{isPending && <button disabled>Adding blog...</button>}
			</form>
		</div>
	);
}

export default Create;