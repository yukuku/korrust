package inherited.same
import inherited.library.JavaTypes.Base
class Result
class Derived: Base() { fun identity(value: Result): Result = value }
