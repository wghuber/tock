#!/usr/bin/env bash

# Install clippy if it is not already preset.
if ! rustup component list | grep 'clippy.*(installed)' -q; then
	rustup component add clippy
fi

# Clippy lints we turn off.
#
# - `clippy::redundant_field_names`: When initializing a struct there is no
#   need to specify the field that is being initialized if the name of the
#   variable it is being set to is the same as the field. However, the clarity
#   of being explicit is nice, and it is strange to have some fields explicitly
#   set and other implicitly set.
#
# - `clippy::unreadable_literal`: Large numbers can have a "_" in the middle of
#   them. Sometimes it is nice, sometimes it just isn't helpful.
#
# - `clippy::too_many_arguments`: Sometimes our functions just need many
#   arguments, and this lint just isn't that helpful.
#
# - `clippy::redundant_pattern_matching`: This checks to see if multiple
#   branches of if statements can be combined. While maybe this is useful, often
#   the branches are separated because they are complicated, and merging them
#   does not help code readability.
#
# - `clippy::type_complexity`: Some of our types will be complex, and that is
#   ok.

CLIPPY_ARGS="
-A clippy::redundant_field_names
-A clippy::unreadable_literal
-A clippy::too_many_arguments
-A clippy::redundant_pattern_matching
-A clippy::type_complexity
"

cargo clippy -- $CLIPPY_ARGS
