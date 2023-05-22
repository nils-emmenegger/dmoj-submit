# dmoj-submit
A CLI tool for submitting to DMOJ.

## Installation
TODO: write this section
## Usage
Before being able to use DMOJ-SUBMIT you must have a DMOJ API token. You can get yours [on the profile edit page on DMOJ](https://dmoj.ca/edit/profile/).  
  
The dmoj-submit program has three operations.
### list-languages
dmoj-submit will list languages supported by DMOJ in alphabetical order.  
Here is how you call it:
```
dmoj-submit list-languages
```

### config
This operation will allow you to configure dmoj-submit to remember your API token and language preferences.  
Here is how you may use it:
```
dmoj-submit config [options]

Options:
  -t, --token TOKEN                        set your API token
  -l, --language EXTENSION:LANGUAGE        set a submission language as a default for a given file extension
```
Here is a example that will set the default token to be `my_token` and will make all python submissions use `PYPY3`:
```
dmoj-submit config -t my_token -l py:PYPY3
```
Previously set defaults may be overwritten by running the command again.
### submit
This is the main operation and raison d'être for dmoj-submit. This operation allows you to submit to DMOJ from your command line.  
Here is how you may use it:
```
dmoj-submit submit [file] [options]

file: this is the file you wish to submit

Options:
  -p, --problem PROBLEM                    specify a problem to submit to
  -t, --token TOKEN                        specify a token to use
  -l, --language LANGUAGE                  specify the language you wish to use
  -v                                       show warnings
  -vv                                      show additional information
```
Here is an example of how one might submit to [hello world](https://dmoj.ca/problem/helloworld):
```
dmoj-submit submit file.py -p helloworld -l pypy3 -t my_token
```
Here is another example of how one could submit to the same problem after using `config` to set a token and language default and naming their file after the problem they wish to submit to:
```
dmoj-submit submit helloworld.py
```
dmoj-submit wil then output their submission results. It could look something like this:
```
Case  #1: AC [0.056s, 56.93MB]
Result: AC
Resources: 0.056s, 55.59MB
Final score: 100/100
```
