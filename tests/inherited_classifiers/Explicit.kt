package inherited.explicit
import inherited.library.JavaTypes.Base
import inherited.library.JavaTypes.Other.Result
class Derived: Base() { fun identity(value: Result): Result = value }
