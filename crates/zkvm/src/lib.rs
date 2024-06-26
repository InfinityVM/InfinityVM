//! Better then your ZKVM.

enum Error {}

//! A ZKVM
trait ZKVM {

	/// Check if the verifying key can be derived from program elf.
	fn is_correct_verifying_key(program_elf: Vec<u8>, program_verifying_key: Vec<u8>) -> bool;

	/// Execute the program and return the raw output.
	///
	/// This does _not_ check that the verifying key is correct.
	fn execute(program_elf: Vec<u8>, ) -> Result<Vec<u8>, Error>

	// methods for losers
	// fn prove()
	// fn verify()
}