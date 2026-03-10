<h3>The Tree</h3>
<script>
    let socket = new WebSocket('ws://127.0.0.1:1236');
    let messages = [];

    socket.onmessage = (event) => {
        messages = [...messages, event.data];
    };

    function sendMessage() {
        socket.send("Hello from Svelte!");
    }
</script>

<button onclick={sendMessage}>Send Message</button>
<ul>
    {#each messages as msg}
        <li>{msg}</li>
    {/each}
</ul>
