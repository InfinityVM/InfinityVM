package db

// DB defines an interface for key/value retrieval and storage.
type DB interface {
	Set([]byte, []byte) error
	Get([]byte) ([]byte, error)
	Delete([]byte) error
	Close() error
}
