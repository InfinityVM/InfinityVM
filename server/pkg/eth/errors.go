package eth

import "fmt"

type FatalClientError struct {
	Message string
}

func (e *FatalClientError) Error() string {
	return fmt.Sprintf("FatalClientError: %s", e.Message)
}

func NewFatalClientError(message string) error {
	return &FatalClientError{Message: message}
}
