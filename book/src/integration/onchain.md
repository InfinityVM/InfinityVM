# Onchain jobs

Onchain job requests are initiated by an event emitted from a smart contract. Typically we expect the request event to be triggered by user interaction. But in some designs, contract callback handling logic for a previous job result may trigger a new request event, effectively creating a continuous loop of requests without user interaction.

![onchain job request](../assets/onchain-job-request.png)