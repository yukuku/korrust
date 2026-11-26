fun consume(): Boolean = acceptNullable(null) == null && nonNullAsNullable(true) == null && generic("ok") == "ok" && checkedStrictNonNull() == "strict"
