#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, sp_runtime::SaturatedConversion, traits::Currency};
	use frame_system::pallet_prelude::*;
	//use pallet_timestamp::*;
	use scale_info::TypeInfo;
	use sp_std::vec::Vec;
	

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;
	}

	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[derive(Clone, Encode, Decode, Eq,PartialEq, RuntimeDebug, TypeInfo)]
	pub enum Type {
		A,
		AAAA,
		MX,
		CNAME,
		IPFS,
	}
	
	impl Default for Type {
		fn default() -> Self {
			Type::A
		}
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
	pub struct Record{
		rr_type: Type,
		ip: Vec<u8>,
		ttl: u32,
	}

	// The pallet's runtime storage items.
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn domains)]
	// Learn more about declaring storage items:
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
	pub type Domains<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], (T::AccountId, u128, u128), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub type Accounts<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<[u8; 32]>>;

	#[pallet::storage]
	#[pallet::getter(fn subdomains)]
	pub type SubDomains<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], Vec<[u8; 32]>>;

	#[pallet::storage]
	#[pallet::getter(fn records)]
	pub type Records<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], Vec<Record>>;
	

	#[pallet::storage]
	#[pallet::getter(fn admin)]
	pub type Admin<T: Config> = StorageValue<_,T::AccountId>;
	
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config>{
		pub admin: T::AccountId,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self{
				admin: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T>{
		fn build(&self) {
			Admin::<T>::put(self.admin.clone());
		}
	}

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),

		Register([u8; 32], T::AccountId),
		Withdraw([u8; 32]),
		ExistDomain([u8; 32]),
		NoDomain([u8; 32]),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,

		DomainAlreadyExist,
		NoDomain,
		NotDomainOwner,
		ErrorData,
		ErrorBalance,
	}


	const MIN_LEN: u8 = 4;
	const MIN_DURATION: u128 = 700000000; // ms
	const MAX_DURATION: u128 = 20000000000;
	const PRICE_BASE: u128 = 10;


	fn cost(_domain: [u8; 32], domain_len: u8, duration: u128) -> u128 {
		return duration * ((32 - domain_len) as u128) * PRICE_BASE;
	}

	fn lower(char: &mut u8) {
		if *char >= 65 && *char <= 90 {
			*char = *char + 32;
		}
	}

	fn str(domain: &mut [u8; 32]) -> u8 {
		let mut len: u8 = 0;
		let mut zero: bool = false;
		for i in 0..32 {
			if zero {
				domain[i] = 0;
			}
			else {
				if domain[i] == 0 {
					zero = true;
				}
				else if (domain[i] != 46 && domain[i] < 48) || domain[i] > 122||(domain[i] > 59 && domain[i] < 65)||(domain[i] > 90 && domain[i] < 97){ // '.' : 46; '0' : 48; 'z' : 122
					return 0;
				}
				else {
					len += 1;
					lower(&mut domain[i]);
				}
			}
		}
		return len;
	}

	fn get_top(domain: &[u8; 32], len: u8) -> [u8; 32] {
		let mut top: [u8; 32] = [0; 32];
		let mut pos: i8 = -1;
		for i in (0..(len as usize)).rev() {
			if domain[i] == 46 {
				pos = i as i8;
				break;
			}
		}

		pos += 1;
		let l: usize = (len as i8 - pos) as usize;
		for i in 0..l {
			top[i] = domain[pos as usize];
			pos += 1;
		}
		return top;
	}


	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://substrate.dev/docs/en/knowledgebase/runtime/origin
			let who = ensure_signed(origin)?;

			// Update storage.
			//<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored(something, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			/*
			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue)?,
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}*/
			Ok(())
		}

		#[pallet::weight(1_010_000 + T::DbWeight::get().reads_writes(1,2)) ]
		pub fn register(origin: OriginFor<T>, mut domain: [u8; 32], duration: u128) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			if duration < MIN_DURATION || duration > MAX_DURATION {
				Err(Error::<T>::ErrorData)?
			}
			
			let mut len: u8 = 0;
			let mut zero: bool = false;
			for i in 0..32 {
				if zero {
					domain[i] = 0;
				}
				else {
					if domain[i] == 0 {
						zero = true;
					}
					else if domain[i] < 48 || domain[i] > 122||(domain[i] > 59 && domain[i] < 65)||(domain[i] > 90 && domain[i] < 97){ // '.' : 46; '0' : 48; 'z' : 122
						Err(Error::<T>::ErrorData)?
					}
					else {
						len += 1;
						lower(&mut domain[i]);
					}
				}
			}

			if len < MIN_LEN {
				Err(Error::<T>::ErrorData)?
			}

			Self::exist(origin.clone(), domain.clone())?;

			if <Domains<T>>::contains_key(domain) {
				Err(Error::<T>::DomainAlreadyExist)?
			}

			let cost = cost(domain.clone(), len, duration.clone());
			if T::Currency::free_balance(&who) < cost.saturated_into::<BalanceOf<T>>() {
				Err(Error::<T>::ErrorBalance)?
			}
			
			let admin = <Admin<T>>::get().unwrap();
			T::Currency::transfer(&who, &admin, cost.saturated_into::<BalanceOf<T>>(), frame_support::traits::tokens::ExistenceRequirement::AllowDeath)?;

			let start: u128= <pallet_timestamp::Pallet<T>>::get().saturated_into::<u128>();
			<Domains<T>>::insert(domain.clone(),(who.clone(), start, duration.clone()));
			if !<Accounts<T>>::contains_key(who.clone()) {
				<Accounts<T>>::insert(who.clone(),Vec::<[u8;32]>::new());
			}
			<Accounts<T>>::append(who.clone(), domain.clone()); // mutate

			Self::deposit_event(Event::Register(domain, who));
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2,1)) ]
		pub fn add_subdomain(origin: OriginFor<T>, mut subdomain: [u8; 32]) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let len = str(&mut subdomain);
			if len == 0 {
				Err(Error::<T>::ErrorData)?
			}

			let top = get_top(&subdomain, len);
			Self::exist(origin.clone(), top.clone())?;

			ensure!(<Domains<T>>::contains_key(top.clone()), Error::<T>::NoDomain);
			ensure!(<Domains<T>>::get(top.clone()).0 == who, Error::<T>::NotDomainOwner);		

			if !<SubDomains<T>>::contains_key(top.clone()) {
				<SubDomains<T>>::insert(top.clone(), Vec::<[u8; 32]>::new());
			}
			<SubDomains<T>>::append(top.clone(), subdomain);
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2,1)) ]
		pub fn delete_subdomain(origin: OriginFor<T>, mut subdomain: [u8; 32]) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let len = str(&mut subdomain);
			if len == 0 {
				Err(Error::<T>::ErrorData)?
			}

			let top = get_top(&subdomain, len);
			ensure!(<Domains<T>>::contains_key(top.clone()), Error::<T>::NoDomain);
			ensure!(<Domains<T>>::get(top.clone()).0 == who, Error::<T>::NotDomainOwner);

			ensure!(<SubDomains<T>>::contains_key(top.clone()), Error::<T>::NoDomain);
			
			<SubDomains<T>>::mutate(top.clone(),|x| -> DispatchResult {
				match x {
					Some(arr) => {
						arr.retain(|domain| *domain != subdomain);
						Ok(())
					},
					None => Err(Error::<T>::NoneValue)?
				}
				
			})?;

			<Records<T>>::remove(subdomain.clone());
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(4,1)) ]
		pub fn add_record(origin: OriginFor<T>, mut domain: [u8; 32], rr_type: Type, ip: Vec<u8>, ttl: u32) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let len = str(&mut domain);
			if len == 0 {
				Err(Error::<T>::ErrorData)?
			}

			let top = get_top(&domain, len);
			ensure!(<Domains<T>>::contains_key(top.clone()), Error::<T>::NoDomain);
			ensure!(<Domains<T>>::get(top.clone()).0 == who, Error::<T>::NotDomainOwner);
			ensure!(<SubDomains<T>>::contains_key(top.clone()), Error::<T>::NoDomain);
			ensure!(<SubDomains<T>>::get(top.clone()).unwrap().contains(&domain), Error::<T>::NoDomain);

			Self::exist(origin.clone(), domain.clone())?;
			
			if !<Records<T>>::contains_key(domain.clone()) {
				<Records<T>>::insert(domain.clone(), Vec::<Record>::new());
			}
			<Records<T>>::append(domain.clone(), Record{rr_type, ip, ttl});
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2,1)) ]
		pub fn delete_record(origin: OriginFor<T>, mut domain: [u8; 32], rr_type: Type, ip: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let len = str(&mut domain);
			if len == 0 {
				Err(Error::<T>::ErrorData)?
			}

			let top = get_top(&domain, len);
			ensure!(<Domains<T>>::contains_key(top.clone()), Error::<T>::NoDomain);
			ensure!(<Domains<T>>::get(top.clone()).0 == who, Error::<T>::NotDomainOwner);
			ensure!(<Records<T>>::contains_key(domain.clone()), Error::<T>::NoDomain);
			
			<Records<T>>::mutate(domain.clone(),|x| -> DispatchResult {
				match x {
					Some(arr) => {
						arr.retain(|record| record.rr_type != rr_type || record.ip != ip);
						Ok(())
					},
					None => Err(Error::<T>::NoneValue)?
				}
				
			})?;

			if <Records<T>>::get(domain.clone()).unwrap().is_empty() {
				<Records<T>>::remove(domain.clone());
			}
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(6,4))]
		pub fn withdraw(origin: OriginFor<T>, mut domain: [u8; 32]) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let len = str(&mut domain);
			if len == 0 {
				Err(Error::<T>::ErrorData)?
			}
	
			let top = get_top(&domain, len);
			if <Domains<T>>::contains_key(top.clone()) {
				let info = <Domains<T>>::get(top.clone());
				if info.0 == who || who == <Admin<T>>::get().unwrap() {
					<Accounts<T>>::mutate(who.clone(), |x| -> DispatchResult {
						match x {
							Some(arr) =>{
								arr.retain(|name| *name != top);
								Ok(())
							},
							None => Err(Error::<T>::ErrorData)?
						}
					})?;

					if <SubDomains<T>>::contains_key(top.clone()) {
						for subdomain in <SubDomains<T>>::get(top.clone()).unwrap() {
							<Records<T>>::remove(subdomain);
						}
						<SubDomains<T>>::remove(top.clone());
					}
					
					<Domains<T>>::remove(top.clone());
					if <Accounts<T>>::get(who.clone()).unwrap().is_empty() {
						<Accounts<T>>::remove(who.clone());
					}
					Self::deposit_event(Event::Withdraw(top));
					Ok(())
				}
				else {
					Err(Error::<T>::ErrorData)?
				}
			}
			else {
				Ok(())
			}
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2,1))]
		pub fn exist(_origin: OriginFor<T>, mut domain: [u8; 32]) -> DispatchResult {
			let len = str(&mut domain);
			if len == 0 {
				Err(Error::<T>::ErrorData)?
			}
	
			let top = get_top(&domain, len);
			if <Domains<T>>::contains_key(top.clone()) {
				let info = <Domains<T>>::get(top.clone());
				let now: u128 = <pallet_timestamp::Pallet<T>>::get().saturated_into::<u128>();
				if info.1 + info.2 <= now {
					Self::withdraw(frame_system::RawOrigin::Root.into(), domain)?;
					//Ok(())
				}
				else {
					Self::deposit_event(Event::ExistDomain(domain));
				}
			}
			else {
				Self::deposit_event(Event::NoDomain(domain));
			}
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(4,2))]
		pub fn transfer(origin: OriginFor<T>, mut domain: [u8; 32], to: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			
			let len = str(&mut domain);
			if len == 0 {
				Err(Error::<T>::ErrorData)?
			}

			Self::exist(origin.clone(), domain.clone())?;
	
			ensure!(<Domains<T>>::contains_key(domain.clone()), Error::<T>::NoDomain);
			ensure!(<Domains<T>>::get(domain.clone()).0 == who, Error::<T>::NotDomainOwner);

			<Accounts<T>>::mutate(who.clone(), |x| -> DispatchResult {
				match x {
					Some(arr) => {
						arr.retain(|name| *name != domain);
						Ok(())
					},
					None => Err(Error::<T>::ErrorData)?
				}
			})?;
			if !<Accounts<T>>::contains_key(to.clone()) {
				<Accounts<T>>::insert(to.clone(),Vec::<[u8;32]>::new());
			}
			<Accounts<T>>::append(to.clone(), domain.clone());
			
			<Domains<T>>::mutate(domain.clone(),|x| -> DispatchResult {
				x.0 = to;
				Ok(())
			})?;
			Ok(())
		}
	}
}
