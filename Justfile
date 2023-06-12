set shell := ["bash", "-c"]
set positional-arguments

publish:
    @./tools/publish.sh $1


