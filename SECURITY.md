# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately through GitHub Security Advisories at https://github.com/excelano/xaddr/security/advisories/new. If you would rather not use GitHub, email david.anderson@excelano.com instead. I aim to respond within seven days.

Please do not open public issues for security problems.

## Supported versions

The latest 0.x release receives security fixes. Older versions are not supported.

## What xaddr can access

Nothing. xaddr is a library with no dependencies. It reads no files, opens no sockets, spawns no processes, reads no environment variables, and has no global state. It takes a string and a caller-supplied table description, and returns indices or an error. Everything it can reach is passed to it by the calling program.

## What xaddr stores

Nothing. There is no cache, no config, no history, and no telemetry. A parsed address lives as long as the value the caller holds.

## What a caller should know

The whole attack surface is untrusted input to `parse`, so that is where the guarantees live. Parsing is total: every input either returns a `Spec` or an `Error`, and no input panics, including empty strings, unterminated brackets, absurdly long column letters, invalid UTF-8 boundaries in bracketed names, and numbers too large for `usize`. Column letters past `MAX_COL` are rejected rather than allowed to overflow. There is no recursion in the parser, so no input can exhaust the stack.

Resolution is bounded by the table the caller describes. `Bounds::Strict` rejects any address running past the edge, and `Bounds::Clamp` truncates it. Neither can return an index outside the `ncols` and `nrows` the caller reported, so a resolved address is always safe to use as an index into a table of the size that was described. A `Grid` implementation that misreports its own dimensions is the one way to break that, and it is the caller's to get right.

One resource note worth stating plainly: an address like `A:XFD` combined with a large table resolves to a proportionally large result, since `cells` returns one entry per addressed cell. That is a faithful answer to what was asked, not a defect, but a program accepting addresses from an untrusted source should bound the table or check the resolved size before expanding it.
