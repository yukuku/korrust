import interop.visibility.libraryRestricted
fun bad(): Int { libraryRestricted = 9; return libraryRestricted }
