package db

// DB defines an interface for key/value retrieval and storage.
type DB interface {
	// Set stores the key/value pair in the database.
	Set([]byte, []byte) error
	// Get retrieves the value for the given key.
	Get([]byte) ([]byte, error)
	// Has checks if the key exists in the database.
	Has(key []byte) (bool, error)
	// Delete removes the key/value pair from the database.
	Delete([]byte) error
	// Close closes the database connection.
	Close() error
}
