if [[ ! -d proto-mav-gen ]]
then
	git clone git@github.com:eucleo/proto-mav-gen.git
fi

# Force a rebuild, that is where the work is done.
rm -rf target

rm -rf proto-mav-gen/proto
rm -rf proto-mav-gen/src

cargo build

cd proto-mav-gen
cargo fmt
cd ..

echo "******************************************************"
echo "Check the changes in proto-mav-gen and commit if good."
