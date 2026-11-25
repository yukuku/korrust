package functiondata

data class Handlers(val action: (Int) -> Int, val optional: ((String) -> String)?)
